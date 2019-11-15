#[cfg(test)]
use fnv::FnvHashMap;
#[cfg(test)]
use juniper::Object;
use juniper::{DefaultScalarValue, GraphQLObject};

#[cfg(test)]
use juniper::{self, execute, EmptyMutation, GraphQLType, RootNode, Value, Variables};

#[derive(GraphQLObject, Debug, PartialEq)]
#[graphql(
    name = "MyObj",
    description = "obj descr",
    scalar = DefaultScalarValue
)]
struct Obj {
    regular_field: bool,
    #[graphql(
        name = "renamedField",
        description = "descr",
        deprecated = "field deprecation"
    )]
    c: i32,
}

#[derive(GraphQLObject, Debug, PartialEq)]
#[graphql(scalar = DefaultScalarValue)]
struct Nested {
    obj: Obj,
}

struct Query;

/// Object comment.
#[derive(GraphQLObject, Debug, PartialEq)]
struct DocComment {
    /// Field comment.
    regular_field: bool,
}

/// Doc 1.\
/// Doc 2.
///
/// Doc 4.
#[derive(GraphQLObject, Debug, PartialEq)]
struct MultiDocComment {
    /// Field 1.
    /// Field 2.
    regular_field: bool,
}

/// This is not used as the description.
#[derive(GraphQLObject, Debug, PartialEq)]
#[graphql(description = "obj override")]
struct OverrideDocComment {
    /// This is not used as the description.
    #[graphql(description = "field override")]
    regular_field: bool,
}

#[derive(GraphQLObject, Debug, PartialEq)]
struct WithLifetime<'a> {
    regular_field: &'a i32,
}

#[derive(GraphQLObject, Debug, PartialEq)]
struct SkippedFieldObj {
    regular_field: bool,
    #[graphql(skip)]
    skipped: i32,
}

struct Context;
impl juniper::Context for Context {}

#[derive(GraphQLObject, Debug)]
#[graphql(Context = Context)]
struct WithCustomContext {
    a: bool,
}

#[juniper::object]
impl Query {
    fn obj() -> Obj {
        Obj {
            regular_field: true,
            c: 22,
        }
    }

    fn nested() -> Nested {
        Nested {
            obj: Obj {
                regular_field: false,
                c: 333,
            },
        }
    }

    fn doc() -> DocComment {
        DocComment {
            regular_field: true,
        }
    }

    fn multi_doc() -> MultiDocComment {
        MultiDocComment {
            regular_field: true,
        }
    }

    fn override_doc() -> OverrideDocComment {
        OverrideDocComment {
            regular_field: true,
        }
    }

    fn skipped_field_obj() -> SkippedFieldObj {
        SkippedFieldObj {
            regular_field: false,
            skipped: 42,
        }
    }
}

#[test]
fn test_doc_comment_simple() {
    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = DocComment::meta(&(), &mut registry);
    assert_eq!(meta.description(), Some(&"Object comment.".to_string()));

    check_descriptions(
        "DocComment",
        &Value::scalar("Object comment."),
        "regularField",
        &Value::scalar("Field comment."),
    );
}

#[test]
fn test_multi_doc_comment() {
    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = MultiDocComment::meta(&(), &mut registry);
    assert_eq!(
        meta.description(),
        Some(&"Doc 1. Doc 2.\n\nDoc 4.".to_string())
    );

    check_descriptions(
        "MultiDocComment",
        &Value::scalar("Doc 1. Doc 2.\n\nDoc 4."),
        "regularField",
        &Value::scalar("Field 1.\nField 2."),
    );
}

#[test]
fn test_doc_comment_override() {
    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = OverrideDocComment::meta(&(), &mut registry);
    assert_eq!(meta.description(), Some(&"obj override".to_string()));

    check_descriptions(
        "OverrideDocComment",
        &Value::scalar("obj override"),
        "regularField",
        &Value::scalar("field override"),
    );
}

#[test]
fn test_derived_object() {
    assert_eq!(
        <Obj as GraphQLType<DefaultScalarValue>>::name(&()),
        Some("MyObj")
    );

    // Verify meta info.
    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = Obj::meta(&(), &mut registry);

    assert_eq!(meta.name(), Some("MyObj"));
    assert_eq!(meta.description(), Some(&"obj descr".to_string()));

    let doc = r#"
        {
            obj {
                regularField
                renamedField
            }
        }"#;

    let schema = RootNode::new(Query, EmptyMutation::<()>::new());

    assert_eq!(
        execute(doc, None, &schema, &Variables::new(), &()),
        Ok((
            Value::object(
                vec![(
                    "obj",
                    Value::object(
                        vec![
                            ("regularField", Value::scalar(true)),
                            ("renamedField", Value::scalar(22)),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[test]
#[should_panic]
fn test_cannot_query_skipped_field() {
    let doc = r#"
        {
            skippedFieldObj {
                skippedField
            }
        }"#;
    let schema = RootNode::new(Query, EmptyMutation::<()>::new());
    execute(doc, None, &schema, &Variables::new(), &()).unwrap();
}

#[test]
fn test_skipped_field_siblings_unaffected() {
    let doc = r#"
        {
            skippedFieldObj {
                regularField
            }
        }"#;
    let schema = RootNode::new(Query, EmptyMutation::<()>::new());
    execute(doc, None, &schema, &Variables::new(), &()).unwrap();
}

#[test]
fn test_derived_object_nested() {
    let doc = r#"
        {
            nested {
                obj {
                    regularField
                    renamedField
                }
            }
        }"#;

    let schema = RootNode::new(Query, EmptyMutation::<()>::new());

    assert_eq!(
        execute(doc, None, &schema, &Variables::new(), &()),
        Ok((
            Value::object(
                vec![(
                    "nested",
                    Value::object(
                        vec![(
                            "obj",
                            Value::object(
                                vec![
                                    ("regularField", Value::scalar(false)),
                                    ("renamedField", Value::scalar(333)),
                                ]
                                .into_iter()
                                .collect(),
                            ),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[cfg(test)]
fn check_descriptions(
    object_name: &str,
    object_description: &Value,
    field_name: &str,
    field_value: &Value,
) {
    let doc = format!(
        r#"
    {{
        __type(name: "{}") {{
            name,
            description,
            fields {{
                name
                description
            }}
        }}
    }}
    "#,
        object_name
    );
    run_type_info_query(&doc, |(type_info, values)| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&Value::scalar(object_name))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(object_description)
        );
        assert!(values.contains(&Value::object(
            vec![
                ("name", Value::scalar(field_name)),
                ("description", field_value.clone()),
            ]
            .into_iter()
            .collect(),
        )));
    });
}

#[cfg(test)]
fn run_type_info_query<F>(doc: &str, f: F)
where
    F: Fn((&Object<DefaultScalarValue>, &Vec<Value>)) -> (),
{
    let schema = RootNode::new(Query, EmptyMutation::<()>::new());

    let (result, errs) =
        execute(doc, None, &schema, &Variables::new(), &()).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:#?}", result);

    let type_info = result
        .as_object_value()
        .expect("Result is not an object")
        .get_field_value("__type")
        .expect("__type field missing")
        .as_object_value()
        .expect("__type field not an object value");

    let fields = type_info
        .get_field_value("fields")
        .expect("fields field missing")
        .as_list_value()
        .expect("fields not a list");

    f((type_info, fields));
}
