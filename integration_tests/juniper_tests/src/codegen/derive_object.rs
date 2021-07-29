use fnv::FnvHashMap;
use juniper::{
    execute, graphql_object, graphql_value, DefaultScalarValue, EmptyMutation, EmptySubscription,
    GraphQLObject, GraphQLType, Object, Registry, RootNode, Value, Variables,
};

#[derive(GraphQLObject, Debug, PartialEq)]
#[graphql(name = "MyObj", description = "obj descr")]
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
struct Nested {
    obj: Obj,
}

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

#[derive(GraphQLObject, Debug, PartialEq)]
#[graphql(rename_all = "none")]
struct NoRenameObj {
    one_field: bool,
    another_field: i32,
}

struct Context;
impl juniper::Context for Context {}

#[derive(GraphQLObject, Debug)]
#[graphql(context = Context)]
struct WithCustomContext {
    a: bool,
}

struct Query;

#[graphql_object]
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

    fn no_rename_obj() -> NoRenameObj {
        NoRenameObj {
            one_field: true,
            another_field: 146,
        }
    }
}

struct NoRenameQuery;

#[graphql_object(rename_all = "none")]
impl NoRenameQuery {
    fn obj() -> Obj {
        Obj {
            regular_field: false,
            c: 22,
        }
    }

    fn no_rename_obj() -> NoRenameObj {
        NoRenameObj {
            one_field: true,
            another_field: 146,
        }
    }
}

#[tokio::test]
async fn test_doc_comment_simple() {
    let mut registry: Registry = Registry::new(FnvHashMap::default());
    let meta = DocComment::meta(&(), &mut registry);
    assert_eq!(meta.description(), Some("Object comment."));

    check_descriptions(
        "DocComment",
        &graphql_value!("Object comment."),
        "regularField",
        &graphql_value!("Field comment."),
    )
    .await;
}

#[tokio::test]
async fn test_multi_doc_comment() {
    let mut registry: Registry = Registry::new(FnvHashMap::default());
    let meta = MultiDocComment::meta(&(), &mut registry);
    assert_eq!(meta.description(), Some("Doc 1. Doc 2.\n\nDoc 4."));

    check_descriptions(
        "MultiDocComment",
        &graphql_value!("Doc 1. Doc 2.\n\nDoc 4."),
        "regularField",
        &graphql_value!("Field 1.\nField 2."),
    )
    .await;
}

#[tokio::test]
async fn test_doc_comment_override() {
    let mut registry: Registry = Registry::new(FnvHashMap::default());
    let meta = OverrideDocComment::meta(&(), &mut registry);
    assert_eq!(meta.description(), Some("obj override"));

    check_descriptions(
        "OverrideDocComment",
        &graphql_value!("obj override"),
        "regularField",
        &graphql_value!("field override"),
    )
    .await;
}

#[tokio::test]
async fn test_derived_object() {
    assert_eq!(
        <Obj as GraphQLType<DefaultScalarValue>>::name(&()),
        Some("MyObj")
    );

    // Verify meta info.
    let mut registry: Registry = Registry::new(FnvHashMap::default());
    let meta = Obj::meta(&(), &mut registry);

    assert_eq!(meta.name(), Some("MyObj"));
    assert_eq!(meta.description(), Some("obj descr"));

    let doc = r#"{
        obj {
            regularField
            renamedField
        }
    }"#;

    let schema = RootNode::new(
        Query,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    assert_eq!(
        execute(doc, None, &schema, &Variables::new(), &()).await,
        Ok((
            graphql_value!({
                "obj": {
                    "regularField": true,
                    "renamedField": 22,
                },
            }),
            vec![],
        )),
    );
}

#[tokio::test]
#[should_panic]
async fn test_cannot_query_skipped_field() {
    let doc = r#"{
        skippedFieldObj {
            skippedField
        }
    }"#;
    let schema = RootNode::new(
        Query,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );
    execute(doc, None, &schema, &Variables::new(), &())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_skipped_field_siblings_unaffected() {
    let doc = r#"{
        skippedFieldObj {
            regularField
        }
    }"#;
    let schema = RootNode::new(
        Query,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );
    execute(doc, None, &schema, &Variables::new(), &())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_derived_object_nested() {
    let doc = r#"{
        nested {
            obj {
                regularField
                renamedField
            }
        }
    }"#;

    let schema = RootNode::new(
        Query,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    assert_eq!(
        execute(doc, None, &schema, &Variables::new(), &()).await,
        Ok((
            graphql_value!({
                "nested": {
                    "obj": {
                        "regularField": false,
                        "renamedField": 333,
                    },
                },
            }),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_no_rename_root() {
    let doc = r#"{
        no_rename_obj {
            one_field
            another_field
        }
        obj {
            regularField
        }
    }"#;

    let schema = RootNode::new(
        NoRenameQuery,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    assert_eq!(
        execute(doc, None, &schema, &Variables::new(), &()).await,
        Ok((
            graphql_value!({
                "no_rename_obj": {
                    "one_field": true,
                    "another_field": 146,
                },
                "obj": {
                    "regularField": false,
                },
            }),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_no_rename_obj() {
    let doc = r#"{
        noRenameObj {
            one_field
            another_field
        }
    }"#;

    let schema = RootNode::new(
        Query,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    assert_eq!(
        execute(doc, None, &schema, &Variables::new(), &()).await,
        Ok((
            graphql_value!({
                "noRenameObj": {
                    "one_field": true,
                    "another_field": 146,
                },
            }),
            vec![],
        )),
    );
}

async fn check_descriptions(
    object_name: &str,
    object_description: &Value,
    field_name: &str,
    field_value: &Value,
) {
    let doc = format!(
        r#"{{
            __type(name: "{}") {{
                name
                description
                fields {{
                    name
                    description
                }}
            }}
        }}"#,
        object_name,
    );
    let _result = run_type_info_query(&doc, |(type_info, values)| {
        assert_eq!(
            type_info.get_field_value("name"),
            Some(&graphql_value!(object_name))
        );
        assert_eq!(
            type_info.get_field_value("description"),
            Some(object_description)
        );
        assert!(values.contains(&graphql_value!({
            "name": (field_name.clone()),
            "description": (field_value.clone()),
        })));
    })
    .await;
}

async fn run_type_info_query<F>(doc: &str, f: F)
where
    F: Fn((&Object<DefaultScalarValue>, &Vec<Value>)) -> (),
{
    let schema = RootNode::new(
        Query,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let (result, errs) = execute(doc, None, &schema, &Variables::new(), &())
        .await
        .expect("Execution failed");

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
