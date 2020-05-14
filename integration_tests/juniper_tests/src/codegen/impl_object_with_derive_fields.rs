#[cfg(test)]
use fnv::FnvHashMap;

use juniper::{DefaultScalarValue, GraphQLObjectInfo};

#[cfg(test)]
use juniper::{
    self, execute, EmptyMutation, EmptySubscription, GraphQLType, Registry, RootNode, Value,
    Variables,
};

#[derive(Default)]
struct Context {
    value: String,
}

impl juniper::Context for Context {}

#[derive(GraphQLObjectInfo)]
#[graphql(scalar = DefaultScalarValue)]
struct Obj {
    regular_field: bool,

    #[graphql(name = "renamedField")]
    renamed_field_orig: i32,

    #[graphql(skip)]
    skipped_field: i32,
}

#[juniper::graphql_object(name = "MyObj", description = "obj descr", derive_fields)]
impl Obj {
    fn resolve_field() -> &str {
        "obj::resolve_field"
    }
}

#[derive(GraphQLObjectInfo)]
#[graphql(scalar = DefaultScalarValue)]
struct Nested {
    obj: Obj,
    nested_field: bool,
}

#[juniper::graphql_object(derive_fields)]
impl Nested {
    fn nested_resolve_field() -> &str {
        "nested::resolve_field"
    }
}

#[derive(GraphQLObjectInfo)]
#[graphql(Context = Context, scalar = DefaultScalarValue)]
struct WithContext {
    value: bool,
}

#[juniper::graphql_object(Context = Context, derive_fields)]
impl WithContext {
    fn resolve_field(ctx: &Context) -> &str {
        ctx.value.as_str()
    }
}

// FIXME: Field with lifetime doesn't even work for derive GraphQLObject
//        due to 'cannot infer an appropriate lifetime'.

// #[derive(GraphQLObjectInfo)]
// #[graphql(Context = Context, scalar = DefaultScalarValue)]
// struct WithLifetime<'a> {
//     value: &'a str,
// }

// #[juniper::graphql_object(Context = Context)]
// impl<'a> WithLifetime<'a> {
//     fn custom_field() -> bool {
//         true
//     }
// }

struct Query;

#[juniper::graphql_object(Context = Context, scalar = DefaultScalarValue)]
impl Query {
    fn obj() -> Obj {
        Obj {
            regular_field: true,
            renamed_field_orig: 22,
            skipped_field: 33,
        }
    }

    fn nested(&self) -> Nested {
        Nested {
            obj: Obj {
                regular_field: false,
                renamed_field_orig: 222,
                skipped_field: 333,
            },
            nested_field: true,
        }
    }

    fn with_context(&self) -> WithContext {
        WithContext { value: true }
    }

    // fn with_lifetime(&self) -> WithLifetime<'a> {
    //     WithLifetime { value: "blub" }
    // }
}

#[tokio::test]
async fn test_derived_object_fields() {
    assert_eq!(
        <Obj as GraphQLType<DefaultScalarValue>>::name(&()),
        Some("MyObj")
    );

    // Verify meta info.
    let mut registry = Registry::new(FnvHashMap::default());
    let meta = Obj::meta(&(), &mut registry);

    assert_eq!(meta.name(), Some("MyObj"));
    assert_eq!(meta.description(), Some(&"obj descr".to_string()));

    let doc = r#"
        {
            obj {
                regularField
                renamedField
                resolveField
            }
        }"#;

    let schema = RootNode::new(
        Query,
        EmptyMutation::<Context>::new(),
        EmptySubscription::<Context>::new(),
    );

    let context = Context {
        value: String::from("context value"),
    };

    assert_eq!(
        execute(doc, None, &schema, &Variables::new(), &context).await,
        Ok((
            Value::object(
                vec![(
                    "obj",
                    Value::object(
                        vec![
                            ("regularField", Value::scalar(true)),
                            ("renamedField", Value::scalar(22)),
                            ("resolveField", Value::scalar("obj::resolve_field")),
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

#[tokio::test]
#[should_panic]
async fn test_cannot_query_skipped_field() {
    let doc = r#"
        {
            obj {
                skippedField
            }
        }"#;
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Context>::new(),
        EmptySubscription::<Context>::new(),
    );
    let context = Context {
        value: String::from("context value"),
    };
    execute(doc, None, &schema, &Variables::new(), &context)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_derived_object_fields_nested() {
    let doc = r#"
        {
            nested {
                obj {
                    regularField
                    renamedField
                    resolveField
                }
                nestedField
                nestedResolveField
            }
        }"#;

    let schema = RootNode::new(
        Query,
        EmptyMutation::<Context>::new(),
        EmptySubscription::<Context>::new(),
    );

    let context = Context {
        value: String::from("context value"),
    };

    assert_eq!(
        execute(doc, None, &schema, &Variables::new(), &context).await,
        Ok((
            Value::object(
                vec![(
                    "nested",
                    Value::object(
                        vec![
                            (
                                "obj",
                                Value::object(
                                    vec![
                                        ("regularField", Value::scalar(false)),
                                        ("renamedField", Value::scalar(222)),
                                        ("resolveField", Value::scalar("obj::resolve_field")),
                                    ]
                                    .into_iter()
                                    .collect()
                                ),
                            ),
                            ("nestedField", Value::scalar(true)),
                            ("nestedResolveField", Value::scalar("nested::resolve_field"))
                        ]
                        .into_iter()
                        .collect()
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_field_resolver_with_context() {
    let doc = r#"
        {
            withContext {
                value
                resolveField
            }
        }"#;

    let schema = RootNode::new(
        Query,
        EmptyMutation::<Context>::new(),
        EmptySubscription::<Context>::new(),
    );

    let context = Context {
        value: String::from("context value"),
    };

    assert_eq!(
        execute(doc, None, &schema, &Variables::new(), &context).await,
        Ok((
            Value::object(
                vec![(
                    "withContext",
                    Value::object(
                        vec![
                            ("value", Value::scalar(true)),
                            ("resolveField", Value::scalar("context value")),
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

#[tokio::test]
#[should_panic]
async fn test_duplicate_object_field() {
    #[derive(GraphQLObjectInfo)]
    #[graphql(scalar = DefaultScalarValue)]
    struct TestObject {
        value: bool,
    }

    #[juniper::graphql_object(derive_fields)]
    impl TestObject {
        fn value() -> bool {
            true
        }
    }

    struct TestQuery;

    #[juniper::graphql_object(scalar = DefaultScalarValue)]
    impl TestQuery {
        fn test(&self) -> TestObject {
            TestObject { value: false }
        }
    }

    let doc = r#"
        {
            test {
                value
            }
        }"#;
    let schema = RootNode::new(
        TestQuery,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );
    execute(doc, None, &schema, &Variables::new(), &())
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic]
async fn test_duplicate_object_field_with_custom_name() {
    #[derive(GraphQLObjectInfo)]
    #[graphql(scalar = DefaultScalarValue)]
    struct TestObject {
        #[graphql(name = "renamed")]
        value: bool,
    }

    #[juniper::graphql_object(derive_fields)]
    impl TestObject {
        fn renamed() -> bool {
            true
        }
    }

    struct TestQuery;

    #[juniper::graphql_object(scalar = DefaultScalarValue)]
    impl TestQuery {
        fn test(&self) -> TestObject {
            TestObject { value: false }
        }
    }

    let doc = r#"
        {
            test {
                renamed
            }
        }"#;
    let schema = RootNode::new(
        TestQuery,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );
    execute(doc, None, &schema, &Variables::new(), &())
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic]
async fn test_duplicate_field_resolver_with_custom_name() {
    #[derive(GraphQLObjectInfo)]
    #[graphql(scalar = DefaultScalarValue)]
    struct TestObject {
        value: bool,
    }

    #[juniper::graphql_object(derive_fields)]
    impl TestObject {
        #[graphql(name = "value")]
        fn renamed() -> bool {
            true
        }
    }

    struct TestQuery;

    #[juniper::graphql_object(scalar = DefaultScalarValue)]
    impl TestQuery {
        fn test(&self) -> TestObject {
            TestObject { value: false }
        }
    }

    let doc = r#"
        {
            test {
                value
            }
        }"#;
    let schema = RootNode::new(
        TestQuery,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );
    execute(doc, None, &schema, &Variables::new(), &())
        .await
        .unwrap();
}
