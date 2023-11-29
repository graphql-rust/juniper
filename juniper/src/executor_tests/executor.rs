mod field_execution {
    use crate::{
        graphql_object, graphql_value, graphql_vars,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
    };

    struct DataType;
    struct DeepDataType;

    #[graphql_object]
    impl DataType {
        fn a() -> &'static str {
            "Apple"
        }
        fn b() -> &'static str {
            "Banana"
        }
        fn c() -> &'static str {
            "Cookie"
        }
        fn d() -> &'static str {
            "Donut"
        }
        fn e() -> &'static str {
            "Egg"
        }
        fn f() -> &'static str {
            "Fish"
        }

        fn pic(size: Option<i32>) -> String {
            format!("Pic of size: {}", size.unwrap_or(50))
        }

        fn deep() -> DeepDataType {
            DeepDataType
        }
    }

    #[graphql_object]
    impl DeepDataType {
        fn a() -> &'static str {
            "Already Been Done"
        }
        fn b() -> &'static str {
            "Boring"
        }
        fn c() -> Vec<Option<&'static str>> {
            vec![Some("Contrived"), None, Some("Confusing")]
        }

        fn deeper() -> Vec<Option<DataType>> {
            vec![Some(DataType), None, Some(DataType)]
        }
    }

    #[tokio::test]
    async fn test() {
        let schema = RootNode::new(
            DataType,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
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
            }
        ";
        let vars = graphql_vars! {"size": 100};

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {result:#?}");

        assert_eq!(
            result,
            graphql_value!({
                "a": "Apple",
                "b": "Banana",
                "x": "Cookie",
                "d": "Donut",
                "e": "Egg",
                "f": "Fish",
                "pic": "Pic of size: 100",
                "deep": {
                    "a": "Already Been Done",
                    "b": "Boring",
                    "c": ["Contrived", null, "Confusing"],
                    "deeper": [
                        {
                            "a": "Apple",
                            "b": "Banana",
                        },
                        null,
                        {
                            "a": "Apple",
                            "b": "Banana",
                        },
                    ],
                },
            }),
        );
    }
}

mod merge_parallel_fragments {
    use crate::{
        graphql_object, graphql_value, graphql_vars,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
    };

    struct Type;

    #[graphql_object]
    impl Type {
        fn a() -> &'static str {
            "Apple"
        }
        fn b() -> &'static str {
            "Banana"
        }
        fn c() -> &'static str {
            "Cherry"
        }
        fn deep() -> Type {
            Type
        }
    }

    #[tokio::test]
    async fn test() {
        let schema = RootNode::new(
            Type,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"
            { a, ...FragOne, ...FragTwo }
            fragment FragOne on Type {
                b
                deep { b, deeper: deep { b } }
            }
            fragment FragTwo on Type {
                c
                deep { c, deeper: deep { c } }
            }
        ";
        let vars = graphql_vars! {};

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {result:#?}");

        assert_eq!(
            result,
            graphql_value!({
                "a": "Apple",
                "b": "Banana",
                "deep": {
                    "b": "Banana",
                    "deeper": {
                        "b": "Banana",
                        "c": "Cherry",
                    },
                    "c": "Cherry",
                },
                "c": "Cherry",
            }),
        );
    }
}

mod merge_parallel_inline_fragments {
    use crate::{
        graphql_object, graphql_value, graphql_vars,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
    };

    struct Type;
    struct Other;

    #[graphql_object]
    impl Type {
        fn a() -> &'static str {
            "Apple"
        }
        fn b() -> &'static str {
            "Banana"
        }
        fn c() -> &'static str {
            "Cherry"
        }
        fn deep() -> Type {
            Type
        }
        fn other() -> Vec<Other> {
            vec![Other, Other]
        }
    }

    #[graphql_object]
    impl Other {
        fn a() -> &'static str {
            "Apple"
        }
        fn b() -> &'static str {
            "Banana"
        }
        fn c() -> &'static str {
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
        let schema = RootNode::new(
            Type,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
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
            }
        ";
        let vars = graphql_vars! {};

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {result:#?}");

        assert_eq!(
            result,
            graphql_value!({
                "a": "Apple",
                "b": "Banana",
                "deep": {
                    "b": "Banana",
                    "deeper": [{
                        "deepest": {
                            "b": "Banana",
                            "c": "Cherry",
                        },
                    }, {
                        "deepest": {
                            "b": "Banana",
                            "c": "Cherry",
                        },
                    }],
                    "c": "Cherry",
                },
                "c": "Cherry",
            }),
        );
    }
}

mod threads_context_correctly {
    use crate::{
        executor::Context,
        graphql_object, graphql_value, graphql_vars,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
    };

    struct Schema;

    struct TestContext {
        value: String,
    }

    impl Context for TestContext {}

    #[graphql_object(context = TestContext)]
    impl Schema {
        fn a(context: &TestContext) -> String {
            context.value.clone()
        }
    }

    #[tokio::test]
    async fn test() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<TestContext>::new(),
            EmptySubscription::<TestContext>::new(),
        );
        let doc = r"{ a }";
        let vars = graphql_vars! {};

        let (result, errs) = crate::execute(
            doc,
            None,
            &schema,
            &vars,
            &TestContext {
                value: "Context value".into(),
            },
        )
        .await
        .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {result:#?}");

        assert_eq!(result, graphql_value!({"a": "Context value"}));
    }
}

mod dynamic_context_switching {
    use indexmap::IndexMap;

    use crate::{
        executor::{Context, ExecutionError, FieldError, FieldResult},
        graphql_object, graphql_value, graphql_vars,
        parser::SourcePosition,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
        Executor, ScalarValue,
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

    #[graphql_object(context = OuterContext)]
    impl Schema {
        fn item_opt<'e, S: ScalarValue>(
            executor: &'e Executor<'_, '_, OuterContext, S>,
            _context: &OuterContext,
            key: i32,
        ) -> Option<(&'e InnerContext, ItemRef)> {
            executor.context().items.get(&key).map(|c| (c, ItemRef))
        }

        fn item_res(context: &OuterContext, key: i32) -> FieldResult<(&InnerContext, ItemRef)> {
            let res = context
                .items
                .get(&key)
                .ok_or(format!("Could not find key {key}"))
                .map(|c| (c, ItemRef))?;
            Ok(res)
        }

        fn item_res_opt(
            context: &OuterContext,
            key: i32,
        ) -> FieldResult<Option<(&InnerContext, ItemRef)>> {
            if key > 100 {
                Err(format!("Key too large: {key}"))?;
            }
            Ok(context.items.get(&key).map(|c| (c, ItemRef)))
        }

        fn item_always(context: &OuterContext, key: i32) -> (&InnerContext, ItemRef) {
            context.items.get(&key).map(|c| (c, ItemRef)).unwrap()
        }
    }

    #[graphql_object(context = InnerContext)]
    impl ItemRef {
        fn value(context: &InnerContext) -> String {
            context.value.clone()
        }
    }

    #[tokio::test]
    async fn test_opt() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<OuterContext>::new(),
            EmptySubscription::<OuterContext>::new(),
        );
        let doc = r"{ first: itemOpt(key: 0) { value }, missing: itemOpt(key: 2) { value } }";
        let vars = graphql_vars! {};

        let ctx = OuterContext {
            items: vec![
                (
                    0,
                    InnerContext {
                        value: "First value".into(),
                    },
                ),
                (
                    1,
                    InnerContext {
                        value: "Second value".into(),
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

        println!("Result: {result:#?}");

        assert_eq!(
            result,
            graphql_value!({
                "first": {"value": "First value"},
                "missing": null,
            }),
        );
    }

    #[tokio::test]
    async fn test_res_success() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<OuterContext>::new(),
            EmptySubscription::<OuterContext>::new(),
        );
        let doc = r"{
            first: itemRes(key: 0) { value }
        }";
        let vars = graphql_vars! {};

        let ctx = OuterContext {
            items: vec![
                (
                    0,
                    InnerContext {
                        value: "First value".into(),
                    },
                ),
                (
                    1,
                    InnerContext {
                        value: "Second value".into(),
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

        println!("Result: {result:#?}");

        assert_eq!(result, graphql_value!({"first": {"value": "First value"}}));
    }

    #[tokio::test]
    async fn test_res_fail() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<OuterContext>::new(),
            EmptySubscription::<OuterContext>::new(),
        );
        let doc = r"{
            missing: itemRes(key: 2) { value }
        }";
        let vars = graphql_vars! {};

        let ctx = OuterContext {
            items: vec![
                (
                    0,
                    InnerContext {
                        value: "First value".into(),
                    },
                ),
                (
                    1,
                    InnerContext {
                        value: "Second value".into(),
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
                SourcePosition::new(14, 1, 12),
                &["missing"],
                FieldError::new("Could not find key 2", graphql_value!(null)),
            )],
        );

        println!("Result: {result:#?}");

        assert_eq!(result, graphql_value!(null));
    }

    #[tokio::test]
    async fn test_res_opt() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<OuterContext>::new(),
            EmptySubscription::<OuterContext>::new(),
        );
        let doc = r"{
            first: itemResOpt(key: 0) { value }
            missing: itemResOpt(key: 2) { value }
            tooLarge: itemResOpt(key: 200) { value }
        }";
        let vars = graphql_vars! {};

        let ctx = OuterContext {
            items: vec![
                (
                    0,
                    InnerContext {
                        value: "First value".into(),
                    },
                ),
                (
                    1,
                    InnerContext {
                        value: "Second value".into(),
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
                SourcePosition::new(112, 3, 12),
                &["tooLarge"],
                FieldError::new("Key too large: 200", graphql_value!(null)),
            )],
        );

        println!("Result: {result:#?}");

        assert_eq!(
            result,
            graphql_value!({
                "first": {"value": "First value"},
                "missing": null,
                "tooLarge": null,
            }),
        );
    }

    #[tokio::test]
    async fn test_always() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<OuterContext>::new(),
            EmptySubscription::<OuterContext>::new(),
        );
        let doc = r"{ first: itemAlways(key: 0) { value } }";
        let vars = graphql_vars! {};

        let ctx = OuterContext {
            items: vec![
                (
                    0,
                    InnerContext {
                        value: "First value".into(),
                    },
                ),
                (
                    1,
                    InnerContext {
                        value: "Second value".into(),
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

        println!("Result: {result:#?}");

        assert_eq!(result, graphql_value!({"first": {"value": "First value"}}));
    }
}

mod propagates_errors_to_nullable_fields {
    use crate::{
        executor::{ExecutionError, FieldError, FieldResult, IntoFieldError},
        graphql_object, graphql_value, graphql_vars,
        parser::SourcePosition,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
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

    #[graphql_object]
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

    #[graphql_object]
    impl Inner {
        fn nullable_field() -> Option<Inner> {
            Some(Inner)
        }
        fn non_nullable_field() -> Inner {
            Inner
        }
        fn nullable_error_field() -> FieldResult<Option<&'static str>> {
            Err("Error for nullableErrorField")?
        }
        fn non_nullable_error_field() -> FieldResult<&'static str> {
            Err("Error for nonNullableErrorField")?
        }
        fn custom_error_field() -> Result<&'static str, CustomError> {
            Err(CustomError::NotFound)
        }
    }

    #[tokio::test]
    async fn nullable_first_level() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"{ inner { nullableErrorField } }";
        let vars = graphql_vars! {};

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {result:#?}");

        assert_eq!(
            result,
            graphql_value!({"inner": {"nullableErrorField": null}}),
        );

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(10, 0, 10),
                &["inner", "nullableErrorField"],
                FieldError::new("Error for nullableErrorField", graphql_value!(null)),
            )],
        );
    }

    #[tokio::test]
    async fn non_nullable_first_level() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"{ inner { nonNullableErrorField } }";
        let vars = graphql_vars! {};

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {result:#?}");

        assert_eq!(result, graphql_value!(null));

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(10, 0, 10),
                &["inner", "nonNullableErrorField"],
                FieldError::new("Error for nonNullableErrorField", graphql_value!(null)),
            )],
        );
    }

    #[tokio::test]
    async fn custom_error_first_level() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"{ inner { customErrorField } }";
        let vars = graphql_vars! {};

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {result:#?}");

        assert_eq!(result, graphql_value!(null));

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(10, 0, 10),
                &["inner", "customErrorField"],
                FieldError::new("Not Found", graphql_value!({"type": "NOT_FOUND"})),
            )],
        );
    }

    #[tokio::test]
    async fn nullable_nested_level() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"{ inner { nullableField { nonNullableErrorField } } }";
        let vars = graphql_vars! {};

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {result:#?}");

        assert_eq!(result, graphql_value!({"inner": {"nullableField": null}}),);

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(26, 0, 26),
                &["inner", "nullableField", "nonNullableErrorField"],
                FieldError::new("Error for nonNullableErrorField", graphql_value!(null)),
            )],
        );
    }

    #[tokio::test]
    async fn non_nullable_nested_level() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"{ inner { nonNullableField { nonNullableErrorField } } }";
        let vars = graphql_vars! {};

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {result:#?}");

        assert_eq!(result, graphql_value!(null));

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(29, 0, 29),
                &["inner", "nonNullableField", "nonNullableErrorField"],
                FieldError::new("Error for nonNullableErrorField", graphql_value!(null)),
            )],
        );
    }

    #[tokio::test]
    async fn nullable_innermost() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"{ inner { nonNullableField { nullableErrorField } } }";
        let vars = graphql_vars! {};

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {result:#?}");

        assert_eq!(
            result,
            graphql_value!({"inner": {"nonNullableField": {"nullableErrorField": null}}}),
        );

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(29, 0, 29),
                &["inner", "nonNullableField", "nullableErrorField"],
                FieldError::new("Error for nullableErrorField", graphql_value!(null)),
            )],
        );
    }

    #[tokio::test]
    async fn non_null_list() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"{ inners { nonNullableErrorField } }";
        let vars = graphql_vars! {};

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {result:#?}");

        assert_eq!(result, graphql_value!(null));

        assert_eq!(
            errs,
            vec![ExecutionError::new(
                SourcePosition::new(11, 0, 11),
                &["inners", "nonNullableErrorField"],
                FieldError::new("Error for nonNullableErrorField", graphql_value!(null)),
            )],
        );
    }

    #[tokio::test]
    async fn non_null_list_of_nullable() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"{ nullableInners { nonNullableErrorField } }";
        let vars = graphql_vars! {};

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        println!("Result: {result:#?}");

        assert_eq!(
            result,
            graphql_value!({"nullableInners": [null, null, null, null, null]}),
        );

        assert_eq!(
            errs,
            vec![
                ExecutionError::new(
                    SourcePosition::new(19, 0, 19),
                    &["nullableInners", "nonNullableErrorField"],
                    FieldError::new("Error for nonNullableErrorField", graphql_value!(null)),
                ),
                ExecutionError::new(
                    SourcePosition::new(19, 0, 19),
                    &["nullableInners", "nonNullableErrorField"],
                    FieldError::new("Error for nonNullableErrorField", graphql_value!(null)),
                ),
                ExecutionError::new(
                    SourcePosition::new(19, 0, 19),
                    &["nullableInners", "nonNullableErrorField"],
                    FieldError::new("Error for nonNullableErrorField", graphql_value!(null)),
                ),
                ExecutionError::new(
                    SourcePosition::new(19, 0, 19),
                    &["nullableInners", "nonNullableErrorField"],
                    FieldError::new("Error for nonNullableErrorField", graphql_value!(null)),
                ),
                ExecutionError::new(
                    SourcePosition::new(19, 0, 19),
                    &["nullableInners", "nonNullableErrorField"],
                    FieldError::new("Error for nonNullableErrorField", graphql_value!(null)),
                ),
            ],
        );
    }
}

mod named_operations {
    use crate::{
        graphql_object, graphql_value, graphql_vars,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
        GraphQLError,
    };

    struct Schema;

    #[graphql_object]
    impl Schema {
        fn a(p: Option<String>) -> &'static str {
            drop(p);
            "b"
        }
    }

    #[tokio::test]
    async fn uses_inline_operation_if_no_name_provided() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"{ a }";
        let vars = graphql_vars! {};

        let (res, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);
        assert_eq!(res, graphql_value!({"a": "b"}));
    }

    #[tokio::test]
    async fn uses_only_named_operation() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"query Example { a }";
        let vars = graphql_vars! {};

        let (res, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);
        assert_eq!(res, graphql_value!({"a": "b"}));
    }

    #[tokio::test]
    async fn uses_named_operation_if_name_provided() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc =
            r"query Example($p: String!) { first: a(p: $p) } query OtherExample { second: a }";
        let vars = graphql_vars! {};

        let (res, errs) = crate::execute(doc, Some("OtherExample"), &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);
        assert_eq!(res, graphql_value!({"second": "b"}));
    }

    #[tokio::test]
    async fn error_if_multiple_operations_provided_but_no_name() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"query Example { first: a } query OtherExample { second: a }";
        let vars = graphql_vars! {};

        let err = crate::execute(doc, None, &schema, &vars, &())
            .await
            .unwrap_err();

        assert_eq!(err, GraphQLError::MultipleOperationsProvided);
    }

    #[tokio::test]
    async fn error_if_unknown_operation_name_provided() {
        let schema = RootNode::new(
            Schema,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"query Example { first: a } query OtherExample { second: a }";
        let vars = graphql_vars! {};

        let err = crate::execute(doc, Some("UnknownExample"), &schema, &vars, &())
            .await
            .unwrap_err();

        assert_eq!(err, GraphQLError::UnknownOperationName);
    }
}
