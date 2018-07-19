#[cfg(test)]
use fnv::FnvHashMap;
#[cfg(test)]
use indexmap::IndexMap;

#[cfg(test)]
use juniper::{self, execute, EmptyMutation, GraphQLType, RootNode, Value, Variables};

#[derive(GraphQLObject, Debug, PartialEq)]
#[graphql(name = "MyObj", description = "obj descr")]
struct Obj {
    regular_field: bool,
    #[graphql(name = "renamedField", description = "descr", deprecation = "field descr")]
    c: i32,
}

#[derive(GraphQLObject, Debug, PartialEq)]
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

/// Doc 1.
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

graphql_object!(Query: () |&self| {
    field obj() -> Obj {
      Obj{
        regular_field: true,
        c: 22,
      }
    }

    field nested() -> Nested {
        Nested{
            obj: Obj{
                regular_field: false,
                c: 333,
            }
        }
    }

    field doc() -> DocComment {
      DocComment{
        regular_field: true,
      }
    }

    field multi_doc() -> MultiDocComment {
      MultiDocComment{
        regular_field: true,
      }
    }

    field override_doc() -> OverrideDocComment {
      OverrideDocComment{
        regular_field: true,
      }
    }
});

#[test]
fn test_doc_comment() {
    let mut registry = juniper::Registry::new(FnvHashMap::default());
    let meta = DocComment::meta(&(), &mut registry);
    assert_eq!(meta.description(), Some(&"Object comment.".to_string()));

    check_descriptions(
        "DocComment",
        &Value::string("Object comment."),
        "regularField",
        &Value::string("Field comment."),
    );
}

#[test]
fn test_multi_doc_comment() {
    let mut registry = juniper::Registry::new(FnvHashMap::default());
    let meta = MultiDocComment::meta(&(), &mut registry);
    assert_eq!(
        meta.description(),
        Some(&"Doc 1. Doc 2.\nDoc 4.".to_string())
    );

    check_descriptions(
        "MultiDocComment",
        &Value::string("Doc 1. Doc 2.\nDoc 4."),
        "regularField",
        &Value::string("Field 1. Field 2."),
    );
}

#[test]
fn test_doc_comment_override() {
    let mut registry = juniper::Registry::new(FnvHashMap::default());
    let meta = OverrideDocComment::meta(&(), &mut registry);
    assert_eq!(meta.description(), Some(&"obj override".to_string()));

    check_descriptions(
        "OverrideDocComment",
        &Value::string("obj override"),
        "regularField",
        &Value::string("field override"),
    );
}

#[test]
fn test_derived_object() {
    assert_eq!(Obj::name(&()), Some("MyObj"));

    // Verify meta info.
    let mut registry = juniper::Registry::new(FnvHashMap::default());
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
                            ("regularField", Value::boolean(true)),
                            ("renamedField", Value::int(22)),
                        ].into_iter()
                            .collect(),
                    ),
                )].into_iter()
                    .collect()
            ),
            vec![]
        ))
    );
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
                                    ("regularField", Value::boolean(false)),
                                    ("renamedField", Value::int(333)),
                                ].into_iter()
                                    .collect(),
                            ),
                        )].into_iter()
                            .collect(),
                    ),
                )].into_iter()
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
        assert_eq!(type_info.get("name"), Some(&Value::string(object_name)));
        assert_eq!(type_info.get("description"), Some(object_description));
        assert!(
            values.contains(&Value::object(
                vec![
                    ("name", Value::string(field_name)),
                    ("description", field_value.clone()),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[cfg(test)]
fn run_type_info_query<F>(doc: &str, f: F)
where
    F: Fn((&IndexMap<String, Value>, &Vec<Value>)) -> (),
{
    let schema = RootNode::new(Query, EmptyMutation::<()>::new());

    let (result, errs) =
        execute(doc, None, &schema, &Variables::new(), &()).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let type_info = result
        .as_object_value()
        .expect("Result is not an object")
        .get("__type")
        .expect("__type field missing")
        .as_object_value()
        .expect("__type field not an object value");

    let fields = type_info
        .get("fields")
        .expect("fields field missing")
        .as_list_value()
        .expect("fields not a list");

    f((type_info, fields));
}
