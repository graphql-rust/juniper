mod field_execution {
    use crate::{
        ast::InputValue, schema::model::RootNode, types::scalars::EmptyMutation, value::Value,
    };

    struct DataType;
    struct DeepDataType;

    #[crate::graphql_object_internal]
    impl DataType {
        fn a() -> &str {
            "Apple"
        }
        fn b() -> &str {
            "Banana"
        }
        fn c() -> &str {
            "Cookie"
        }
        fn d() -> &str {
            "Donut"
        }
        fn e() -> &str {
            "Egg"
        }
        fn f() -> &str {
            "Fish"
        }

        fn pic(size: Option<i32>) -> String {
            format!("Pic of size: {}", size.unwrap_or(50))
        }

        fn deep() -> DeepDataType {
            DeepDataType
        }
    }

    #[crate::graphql_object_internal]
    impl DeepDataType {
        fn a() -> &str {
            "Already Been Done"
        }
        fn b() -> &str {
            "Boring"
        }
        fn c() -> Vec<Option<&str>> {
            vec![Some("Contrived"), None, Some("Confusing")]
        }

        fn deeper() -> Vec<Option<DataType>> {
            vec![Some(DataType), None, Some(DataType)]
        }
    }

    #[tokio::test]
    async fn test() {
        let schema =
            RootNode::<_, _, crate::DefaultScalarValue>::new(DataType, EmptyMutation::<()>::new());
        let doc = r"
          query Example($size: Int) {
            a,
            b,
            x: c
            ...c
            f
            ...on DataType {
              pic(size: $size)
            }
            deep {
              a
              b
              c
              deeper {
                a
                b
              }
            }
          }

          fragment c on DataType {
            d
            e
          }";

        let vars = vec![("size".to_owned(), InputValue::scalar(100))]
            .into_iter()
            .collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            Value::object(
                vec![
                    ("a", Value::scalar("Apple")),
                    ("b", Value::scalar("Banana")),
                    ("x", Value::scalar("Cookie")),
                    ("d", Value::scalar("Donut")),
                    ("e", Value::scalar("Egg")),
                    ("f", Value::scalar("Fish")),
                    ("pic", Value::scalar("Pic of size: 100")),
                    (
                        "deep",
                        Value::object(
                            vec![
                                ("a", Value::scalar("Already Been Done")),
                                ("b", Value::scalar("Boring")),
                                (
                                    "c",
                                    Value::list(vec![
                                        Value::scalar("Contrived"),
                                        Value::null(),
                                        Value::scalar("Confusing"),
                                    ]),
                                ),
                                (
                                    "deeper",
                                    Value::list(vec![
                                        Value::object(
                                            vec![
                                                ("a", Value::scalar("Apple")),
                                                ("b", Value::scalar("Banana")),
                                            ]
                                            .into_iter()
                                            .collect(),
                                        ),
                                        Value::null(),
                                        Value::object(
                                            vec![
                                                ("a", Value::scalar("Apple")),
                                                ("b", Value::scalar("Banana")),
                                            ]
                                            .into_iter()
                                            .collect(),
                                        ),
                                    ]),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                    ),
                ]
                .into_iter()
                .collect()
            )
        );
    }
}

mod merge_parallel_fragments {
    use crate::{schema::model::RootNode, types::scalars::EmptyMutation, value::Value};

    struct Type;

    #[crate::graphql_object_internal]
    impl Type {
        fn a() -> &str {
            "Apple"
        }
        fn b() -> &str {
            "Banana"
        }
        fn c() -> &str {
            "Cherry"
        }
        fn deep() -> Type {
            Type
        }
    }

    #[tokio::test]
    async fn test() {
        let schema = RootNode::new(Type, EmptyMutation::<()>::new());
        let doc = r"
          { a, ...FragOne, ...FragTwo }
          fragment FragOne on Type {
            b
            deep { b, deeper: deep { b } }
          }
          fragment FragTwo on Type {
            c
            deep { c, deeper: deep { c } }
          }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            Value::object(
                vec![
                    ("a", Value::scalar("Apple")),
                    ("b", Value::scalar("Banana")),
                    (
                        "deep",
                        Value::object(
                            vec![
                                ("b", Value::scalar("Banana")),
                                (
                                    "deeper",
                                    Value::object(
                                        vec![
                                            ("b", Value::scalar("Banana")),
                                            ("c", Value::scalar("Cherry")),
                                        ]
                                        .into_iter()
                                        .collect(),
                                    ),
                                ),
                                ("c", Value::scalar("Cherry")),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                    ),
                    ("c", Value::scalar("Cherry")),
                ]
                .into_iter()
                .collect()
            )
        );
    }
}

mod merge_parallel_inline_fragments {
    use crate::{schema::model::RootNode, types::scalars::EmptyMutation, value::Value};

    struct Type;
    struct Other;

    #[crate::graphql_object_internal]
    impl Type {
        fn a() -> &str {
            "Apple"
        }
        fn b() -> &str {
            "Banana"
        }
        fn c() -> &str {
            "Cherry"
        }
        fn deep() -> Type {
            Type
        }
        fn other() -> Vec<Other> {
            vec![Other, Other]
        }
    }

    #[crate::graphql_object_internal]
    impl Other {
        fn a() -> &str {
            "Apple"
        }
        fn b() -> &str {
            "Banana"
        }
        fn c() -> &str {
            "Cherry"
        }
        fn deep() -> Type {
            Type
        }
        fn other() -> Vec<Other> {
            vec![Other, Other]
        }
    }

    #[tokio::test]
    async fn test() {
        let schema = RootNode::new(Type, EmptyMutation::<()>::new());
        let doc = r"
          { a, ...FragOne }
          fragment FragOne on Type {
            b
            deep: deep {
                b
                deeper: other {
                    deepest: deep {
                        b
                    }
                }

                ... on Type {
                    c
                    deeper: other {
                        deepest: deep {
                            c
                        }
                    }
                }
            }

            c
          }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            Value::object(
                vec![
                    ("a", Value::scalar("Apple")),
                    ("b", Value::scalar("Banana")),
                    (
                        "deep",
                        Value::object(
                            vec![
                                ("b", Value::scalar("Banana")),
                                (
                                    "deeper",
                                    Value::list(vec![
                                        Value::object(
                                            vec![(
                                                "deepest",
                                                Value::object(
                                                    vec![
                                                        ("b", Value::scalar("Banana")),
                                                        ("c", Value::scalar("Cherry")),
                                                    ]
                                                    .into_iter()
                                                    .collect(),
                                                ),
                                            )]
                                            .into_iter()
                                            .collect(),
                                        ),
                                        Value::object(
                                            vec![(
                                                "deepest",
                                                Value::object(
                                                    vec![
                                                        ("b", Value::scalar("Banana")),
                                                        ("c", Value::scalar("Cherry")),
                                                    ]
                                                    .into_iter()
                                                    .collect(),
                                                ),
                                            )]
                                            .into_iter()
                                            .collect(),
                                        ),
                                    ]),
                                ),
                                ("c", Value::scalar("Cherry")),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                    ),
                    ("c", Value::scalar("Cherry")),
                ]
                .into_iter()
                .collect()
            )
        );
    }
}

mod threads_context_correctly {
    use crate::{
        executor::Context, schema::model::RootNode, types::scalars::EmptyMutation, value::Value,
    };

    struct Schema;

    struct TestContext {
        value: String,
    }

    impl Context for TestContext {}

    #[crate::graphql_object_internal(
        Context = TestContext,
    )]
    impl Schema {
        fn a(context: &TestContext) -> String {
            context.value.clone()
        }
    }

    #[tokio::test]
    async fn test() {
        let schema = RootNode::new(Schema, EmptyMutation::<TestContext>::new());
        let doc = r"{ a }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(
            doc,
            None,
            &schema,
            &vars,
            &TestContext {
                value: "Context value".to_owned(),
            },
        )
        .await
        .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            Value::object(
                vec![("a", Value::scalar("Context value"))]
                    .into_iter()
                    .collect()
            )
        );
    }
}

mod dynamic_context_switching {
    use indexmap::IndexMap;

    use crate::{
        executor::{Context, ExecutionError, FieldError, FieldResult},
        parser::SourcePosition,
        schema::model::RootNode,
        types::scalars::EmptyMutation,
        value::Value,
    };

    struct Schema;

    struct InnerContext {
        value: String,
    }

    struct OuterContext {
        items: IndexMap<i32, InnerContext>,
    }

    impl Context for OuterContext {}
    impl Context for InnerContext {}

    struct ItemRef;

    #[crate::graphql_object_internal(Context = OuterContext)]
    impl Schema {
        fn item_opt(_context: &OuterContext, key: i32) -> Option<(&InnerContext, ItemRef)> {
            executor.context().items.get(&key).map(|c| (c, ItemRef))
        }

        fn item_res(context: &OuterContext, key: i32) -> FieldResult<(&InnerContext, ItemRef)> {
            let res = context
                .items
                .get(&key)
                .ok_or(format!("Could not find key {}", key))
                .map(|c| (c, ItemRef))?;
            Ok(res)
        }

        fn item_res_opt(
            context: &OuterContext,
            key: i32,
        ) -> FieldResult<Option<(&InnerContext, ItemRef)>> {
            if key > 100 {
                Err(format!("Key too large: {}", key))?;
            }
            Ok(context.items.get(&key).map(|c| (c, ItemRef)))
        }

        fn item_always(context: &OuterContext, key: i32) -> (&InnerContext, ItemRef) {
            context.items.get(&key).map(|c| (c, ItemRef)).unwrap()
        }
    }

    #[crate::graphql_object_internal(Context = InnerContext)]
    impl ItemRef {
        fn value(context: &InnerContext) -> String {
            context.value.clone()
        }
    }

    #[tokio::test]
    async fn test_opt() {
        let schema = RootNode::new(Schema, EmptyMutation::<OuterContext>::new());
        let doc = r"{ first: itemOpt(key: 0) { value }, missing: itemOpt(key: 2) { value } }";

        let vars = vec![].into_iter().collect();

        let ctx = OuterContext {
            items: vec![
                (
                    0,
                    InnerContext {
                        value: "First value".to_owned(),
                    },
                ),
                (
                    1,
                    InnerContext {
                        value: "Second value".to_owned(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &ctx)
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            Value::object(
                vec![
                    (
                        "first",
                        Value::object(
                            vec![("value", Value::scalar("First value"))]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                    ("missing", Value::null()),
                ]
                .into_iter()
                .collect()
            )
        );
    }

    #[tokio::test]
    async fn test_res_success() {
        let schema = RootNode::new(Schema, EmptyMutation::<OuterContext>::new());
        let doc = r"
          {
            first: itemRes(key: 0) { value }
          }
          ";

        let vars = vec![].into_iter().collect();

        let ctx = OuterContext {
            items: vec![
                (
                    0,
                    InnerContext {
                        value: "First value".to_owned(),
                    },
                ),
                (
                    1,
                    InnerContext {
                        value: "Second value".to_owned(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &ctx)
            .await
            .expect("Execution failed");

        assert_eq!(errs, vec![]);

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            Value::object(
                vec![(
                    "first",
                    Value::object(
                        vec![("value", Value::scalar("First value"))]
                            .into_iter()
                            .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            )
        );
    }

    #[tokio::test]
    async fn test_res_fail() {
        let schema = RootNode::new(Schema, EmptyMutation::<OuterContext>::new());
        let doc = r"
          {
            missing: itemRes(key: 2) { value }
          }
          ";

        let vars = vec![].into_iter().collect();

        let ctx = OuterContext {
            items: vec![
                (
                    0,
                    InnerContext {
                        value: "First value".to_owned(),
                    },
                ),
                (
                    1,
                    InnerContext {
                        value: "Second value".to_owned(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &ctx)
            .await
            .expect("Execution failed");

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(25, 2, 12),
                &["missing"],
                FieldError::new("Could not find key 2", Value::null()),
            )]
        );

        println!("Result: {:#?}", result);

        assert_eq!(result, Value::null());
    }

    #[tokio::test]
    async fn test_res_opt() {
        let schema = RootNode::new(Schema, EmptyMutation::<OuterContext>::new());
        let doc = r"
          {
            first: itemResOpt(key: 0) { value }
            missing: itemResOpt(key: 2) { value }
            tooLarge: itemResOpt(key: 200) { value }
          }
          ";

        let vars = vec![].into_iter().collect();

        let ctx = OuterContext {
            items: vec![
                (
                    0,
                    InnerContext {
                        value: "First value".to_owned(),
                    },
                ),
                (
                    1,
                    InnerContext {
                        value: "Second value".to_owned(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &ctx)
            .await
            .expect("Execution failed");

        assert_eq!(
            errs,
            [ExecutionError::new(
                SourcePosition::new(123, 4, 12),
                &["tooLarge"],
                FieldError::new("Key too large: 200", Value::null()),
            )]
        );

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            Value::object(
                vec![
                    (
                        "first",
                        Value::object(
                            vec![("value", Value::scalar("First value"))]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                    ("missing", Value::null()),
                    ("tooLarge", Value::null()),
                ]
                .into_iter()
                .collect()
            )
        );
    }

    #[tokio::test]
    async fn test_always() {
        let schema = RootNode::new(Schema, EmptyMutation::<OuterContext>::new());
        let doc = r"{ first: itemAlways(key: 0) { value } }";

        let vars = vec![].into_iter().collect();

        let ctx = OuterContext {
            items: vec![
                (
                    0,
                    InnerContext {
                        value: "First value".to_owned(),
                    },
                ),
                (
                    1,
                    InnerContext {
                        value: "Second value".to_owned(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &ctx)
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            Value::object(
                vec![(
                    "first",
                    Value::object(
                        vec![("value", Value::scalar("First value"))]
                            .into_iter()
                            .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            )
        );
    }
}

mod propagates_errors_to_nullable_fields {
    use crate::{
        executor::{ExecutionError, FieldError, FieldResult, IntoFieldError},
        parser::SourcePosition,
        schema::model::RootNode,
        types::scalars::EmptyMutation,
        value::{ScalarValue, Value},
    };

    struct Schema;
    struct Inner;

    enum CustomError {
        NotFound,
    }

    impl<S> IntoFieldError<S> for CustomError
    where
        S: ScalarValue,
    {
        fn into_field_error(self) -> FieldError<S> {
            match self {
                CustomError::NotFound => {
                    let v: Value<S> = graphql_value!({
                        "type": "NOT_FOUND"
                    });
                    FieldError::new("Not Found", v)
                }
            }
        }
    }

    #[crate::graphql_object_internal]
    impl Schema {
        fn inner() -> Inner {
            Inner
        }
        fn inners() -> Vec<Inner> {
            (0..5).map(|_| Inner).collect()
        }
        fn nullable_inners() -> Vec<Option<Inner>> {
            (0..5).map(|_| Some(Inner)).collect()
        }
    }

    #[crate::graphql_object_internal]
    impl Inner {
        fn nullable_field() -> Option<Inner> {
            Some(Inner)
        }
        fn non_nullable_field() -> Inner {
            Inner
        }
        fn nullable_error_field() -> FieldResult<Option<&str>> {
            Err("Error for nullableErrorField")?
        }
        fn non_nullable_error_field() -> FieldResult<&str> {
            Err("Error for nonNullableErrorField")?
        }
        fn custom_error_field() -> Result<&str, CustomError> {
            Err(CustomError::NotFound)
        }
    }

    #[tokio::test]
    async fn nullable_first_level() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"{ inner { nullableErrorField } }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            graphql_value!({ "inner": { "nullableErrorField": None } })
        );

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(10, 0, 10),
                &["inner", "nullableErrorField"],
                FieldError::new("Error for nullableErrorField", Value::null()),
            )]
        );
    }

    #[tokio::test]
    async fn non_nullable_first_level() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"{ inner { nonNullableErrorField } }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {:#?}", result);

        assert_eq!(result, graphql_value!(None));

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(10, 0, 10),
                &["inner", "nonNullableErrorField"],
                FieldError::new("Error for nonNullableErrorField", Value::null()),
            )]
        );
    }

    #[tokio::test]
    async fn custom_error_first_level() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"{ inner { customErrorField } }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {:#?}", result);

        assert_eq!(result, graphql_value!(None));

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(10, 0, 10),
                &["inner", "customErrorField"],
                FieldError::new("Not Found", graphql_value!({ "type": "NOT_FOUND" })),
            )]
        );
    }

    #[tokio::test]
    async fn nullable_nested_level() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"{ inner { nullableField { nonNullableErrorField } } }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            graphql_value!({ "inner": { "nullableField": None } })
        );

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(26, 0, 26),
                &["inner", "nullableField", "nonNullableErrorField"],
                FieldError::new("Error for nonNullableErrorField", Value::null()),
            )]
        );
    }

    #[tokio::test]
    async fn non_nullable_nested_level() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"{ inner { nonNullableField { nonNullableErrorField } } }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {:#?}", result);

        assert_eq!(result, graphql_value!(None));

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(29, 0, 29),
                &["inner", "nonNullableField", "nonNullableErrorField"],
                FieldError::new("Error for nonNullableErrorField", Value::null()),
            )]
        );
    }

    #[tokio::test]
    async fn nullable_innermost() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"{ inner { nonNullableField { nullableErrorField } } }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            graphql_value!({ "inner": { "nonNullableField": { "nullableErrorField": None } } })
        );

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(29, 0, 29),
                &["inner", "nonNullableField", "nullableErrorField"],
                FieldError::new("Error for nullableErrorField", Value::null()),
            )]
        );
    }

    #[tokio::test]
    async fn non_null_list() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"{ inners { nonNullableErrorField } }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {:#?}", result);

        assert_eq!(result, graphql_value!(None));

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(11, 0, 11),
                &["inners", "nonNullableErrorField"],
                FieldError::new("Error for nonNullableErrorField", Value::null()),
            )]
        );
    }

    #[tokio::test]
    async fn non_null_list_of_nullable() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"{ nullableInners { nonNullableErrorField } }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            graphql_value!({ "nullableInners": [None, None, None, None, None] })
        );

        assert_eq!(
            errs,
            vec![
                ExecutionError::new(
                    SourcePosition::new(19, 0, 19),
                    &["nullableInners", "nonNullableErrorField"],
                    FieldError::new("Error for nonNullableErrorField", Value::null()),
                ),
                ExecutionError::new(
                    SourcePosition::new(19, 0, 19),
                    &["nullableInners", "nonNullableErrorField"],
                    FieldError::new("Error for nonNullableErrorField", Value::null()),
                ),
                ExecutionError::new(
                    SourcePosition::new(19, 0, 19),
                    &["nullableInners", "nonNullableErrorField"],
                    FieldError::new("Error for nonNullableErrorField", Value::null()),
                ),
                ExecutionError::new(
                    SourcePosition::new(19, 0, 19),
                    &["nullableInners", "nonNullableErrorField"],
                    FieldError::new("Error for nonNullableErrorField", Value::null()),
                ),
                ExecutionError::new(
                    SourcePosition::new(19, 0, 19),
                    &["nullableInners", "nonNullableErrorField"],
                    FieldError::new("Error for nonNullableErrorField", Value::null()),
                ),
            ]
        );
    }
}

mod named_operations {
    use crate::{
        schema::model::RootNode, types::scalars::EmptyMutation, value::Value, GraphQLError,
    };

    struct Schema;

    #[crate::graphql_object_internal]
    impl Schema {
        fn a(p: Option<String>) -> &str {
            let _ = p;
            "b"
        }
    }

    #[tokio::test]
    async fn uses_inline_operation_if_no_name_provided() {
        let schema =
            RootNode::<_, _, crate::DefaultScalarValue>::new(Schema, EmptyMutation::<()>::new());
        let doc = r"{ a }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        assert_eq!(
            result,
            Value::object(vec![("a", Value::scalar("b"))].into_iter().collect())
        );
    }

    #[tokio::test]
    async fn uses_only_named_operation() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"query Example { a }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        assert_eq!(
            result,
            Value::object(vec![("a", Value::scalar("b"))].into_iter().collect())
        );
    }

    #[tokio::test]
    async fn uses_named_operation_if_name_provided() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc =
            r"query Example($p: String!) { first: a(p: $p) } query OtherExample { second: a }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, Some("OtherExample"), &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        assert_eq!(
            result,
            Value::object(vec![("second", Value::scalar("b"))].into_iter().collect())
        );
    }

    #[tokio::test]
    async fn error_if_multiple_operations_provided_but_no_name() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"query Example { first: a } query OtherExample { second: a }";

        let vars = vec![].into_iter().collect();

        let err = crate::execute(doc, None, &schema, &vars, &())
            .await
            .unwrap_err();

        assert_eq!(err, GraphQLError::MultipleOperationsProvided);
    }

    #[tokio::test]
    async fn error_if_unknown_operation_name_provided() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"query Example { first: a } query OtherExample { second: a }";

        let vars = vec![].into_iter().collect();

        let err = crate::execute(doc, Some("UnknownExample"), &schema, &vars, &())
            .await
            .unwrap_err();

        assert_eq!(err, GraphQLError::UnknownOperationName);
    }
}
