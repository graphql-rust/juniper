//! Checks that the root selection set of a `mutation` operation executes serially, per [§6.3.1]
//! of the GraphQL spec.
//! See [#1398](https://github.com/graphql-rust/juniper/issues/1398) for details.
//!
//! [§6.3.1]: https://spec.graphql.org/September2025/#sec-Normal-and-Serial-Execution

mod spec_example {
    use std::sync::atomic::{AtomicI32, Ordering};

    use juniper::{
        EmptySubscription, GraphQLObject, RootNode, execute, graphql_object, graphql_value,
        graphql_vars,
    };

    #[derive(Default)]
    pub struct Context {
        number: AtomicI32,
    }

    impl juniper::Context for Context {}

    #[derive(GraphQLObject)]
    #[graphql(context = Context)]
    pub struct NumberHolder {
        the_number: i32,
    }

    pub struct Query;

    #[graphql_object(context = Context)]
    impl Query {
        fn some() -> bool {
            true
        }
    }

    pub struct Mutation;

    #[graphql_object(context = Context)]
    impl Mutation {
        async fn change_the_number(new_number: i32, context: &Context) -> NumberHolder {
            context.number.store(new_number, Ordering::SeqCst);
            // Without an actual suspension point the field futures complete on first poll,
            // making concurrent execution indistinguishable from serial.
            tokio::task::yield_now().await;
            NumberHolder {
                the_number: context.number.load(Ordering::SeqCst),
            }
        }
    }

    type Schema = RootNode<Query, Mutation, EmptySubscription<Context>>;

    #[tokio::test]
    async fn root_fields_execute_serially() {
        // language=GraphQL
        let doc = r"
            mutation {
                first: changeTheNumber(newNumber: 1) { theNumber }
                second: changeTheNumber(newNumber: 3) { theNumber }
                third: changeTheNumber(newNumber: 2) { theNumber }
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        assert_eq!(
            execute(doc, None, &schema, &graphql_vars! {}, &Context::default()).await,
            Ok((
                graphql_value!({
                    "first": {"theNumber": 1},
                    "second": {"theNumber": 3},
                    "third": {"theNumber": 2},
                }),
                vec![],
            )),
        );
    }
}

mod execution_order {
    use std::sync::Mutex;

    use juniper::{
        EmptySubscription, RootNode, execute, graphql_object, graphql_value, graphql_vars,
    };

    #[derive(Default)]
    pub struct Context {
        log: Mutex<Vec<&'static str>>,
    }

    impl juniper::Context for Context {}

    impl Context {
        fn push(&self, entry: &'static str) {
            self.log.lock().unwrap().push(entry);
        }

        async fn resolve(&self, start: &'static str, end: &'static str) {
            self.push(start);
            tokio::task::yield_now().await;
            self.push(end);
        }

        fn into_log(self) -> Vec<&'static str> {
            self.log.into_inner().unwrap()
        }
    }

    pub struct Query;

    #[graphql_object(context = Context)]
    impl Query {
        async fn a(context: &Context) -> bool {
            context.resolve("a:start", "a:end").await;
            true
        }

        async fn b(context: &Context) -> bool {
            context.resolve("b:start", "b:end").await;
            true
        }
    }

    pub struct Nested;

    #[graphql_object(context = Context)]
    impl Nested {
        async fn x(context: &Context) -> bool {
            context.resolve("x:start", "x:end").await;
            true
        }

        async fn y(context: &Context) -> bool {
            context.resolve("y:start", "y:end").await;
            true
        }
    }

    pub struct Mutation;

    #[graphql_object(context = Context)]
    impl Mutation {
        async fn a(context: &Context) -> bool {
            context.resolve("a:start", "a:end").await;
            true
        }

        async fn b(context: &Context) -> bool {
            context.resolve("b:start", "b:end").await;
            true
        }

        async fn failing(context: &Context) -> Result<bool, &'static str> {
            context.push("failing:start");
            tokio::task::yield_now().await;
            Err("this field always errors")
        }

        async fn nullable_failing(context: &Context) -> Result<Option<bool>, &'static str> {
            context
                .resolve("nullableFailing:start", "nullableFailing:end")
                .await;
            Err("this field always errors")
        }

        async fn nested() -> Nested {
            Nested
        }
    }

    type Schema = RootNode<Query, Mutation, EmptySubscription<Context>>;

    #[tokio::test]
    async fn non_null_root_field_failure_stops_later_fields() {
        // language=GraphQL
        let doc = r"
            mutation {
                a
                failing
                b
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        let ctx = Context::default();
        let (res, errors) = execute(doc, None, &schema, &graphql_vars! {}, &ctx)
            .await
            .unwrap();

        assert_eq!(res, graphql_value!(null));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path(), ["failing"]);

        let log = ctx.into_log();

        assert_eq!(
            &log[..2],
            ["a:start", "a:end"],
            "root fields must execute serially, got {log:?}",
        );
        assert!(
            !log.iter().any(|entry| entry.starts_with("b:")),
            "`b` must not start once `failing` errors, got {log:?}",
        );
    }

    #[tokio::test]
    async fn nullable_root_field_failure_does_not_stop_later_fields() {
        // language=GraphQL
        let doc = r"
            mutation {
                nullableFailing
                b
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        let ctx = Context::default();
        let (res, errors) = execute(doc, None, &schema, &graphql_vars! {}, &ctx)
            .await
            .unwrap();

        assert_eq!(res, graphql_value!({"nullableFailing": null, "b": true}));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path(), ["nullableFailing"]);

        let log = ctx.into_log();

        assert_eq!(
            log,
            [
                "nullableFailing:start",
                "nullableFailing:end",
                "b:start",
                "b:end",
            ],
            "root fields must execute serially",
        );
    }

    #[tokio::test]
    async fn untyped_inline_fragment_executes_in_document_order() {
        // language=GraphQL
        let doc = r"
            mutation {
                a
                ... {
                    b
                }
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        let ctx = Context::default();
        let (_, errors) = execute(doc, None, &schema, &graphql_vars! {}, &ctx)
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(
            ctx.into_log(),
            ["a:start", "a:end", "b:start", "b:end"],
            "root fields must execute serially",
        );
    }

    #[tokio::test]
    async fn skipped_root_field_is_not_resolved() {
        // language=GraphQL
        let doc = r"
            mutation {
                a @skip(if: true)
                b
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        let ctx = Context::default();
        let (res, errors) = execute(doc, None, &schema, &graphql_vars! {}, &ctx)
            .await
            .unwrap();

        assert_eq!(res, graphql_value!({"b": true}));
        assert_eq!(errors.len(), 0);
        assert_eq!(
            ctx.into_log(),
            ["b:start", "b:end"],
            "a skipped root field must not resolve",
        );
    }

    #[tokio::test]
    async fn skipped_fragment_spread_is_not_resolved() {
        // language=GraphQL
        let doc = r"
            mutation {
                a
                ...F @skip(if: true)
            }

            fragment F on Mutation {
                b
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        let ctx = Context::default();
        let (res, errors) = execute(doc, None, &schema, &graphql_vars! {}, &ctx)
            .await
            .unwrap();

        assert_eq!(res, graphql_value!({"a": true}));
        assert_eq!(errors.len(), 0);
        assert_eq!(
            ctx.into_log(),
            ["a:start", "a:end"],
            "a skipped fragment spread must not resolve",
        );
    }

    #[tokio::test]
    async fn skipped_inline_fragment_is_not_resolved() {
        // language=GraphQL
        let doc = r"
            mutation {
                a
                ... on Mutation @skip(if: true) {
                    b
                }
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        let ctx = Context::default();
        let (res, errors) = execute(doc, None, &schema, &graphql_vars! {}, &ctx)
            .await
            .unwrap();

        assert_eq!(res, graphql_value!({"a": true}));
        assert_eq!(errors.len(), 0);
        assert_eq!(
            ctx.into_log(),
            ["a:start", "a:end"],
            "a skipped inline fragment must not resolve",
        );
    }

    #[tokio::test]
    async fn nested_selections_resolve_concurrently() {
        // language=GraphQL
        let doc = r"
            mutation {
                nested {
                    x
                    y
                }
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        let ctx = Context::default();
        let (_, errors) = execute(doc, None, &schema, &graphql_vars! {}, &ctx)
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);

        let log = ctx.into_log();

        assert_eq!(
            &log[..2],
            ["x:start", "y:start"],
            "sub-selections of a root `mutation` field must not be serialized, got {log:?}",
        );
    }

    #[tokio::test]
    async fn query_root_fields_resolve_concurrently() {
        // language=GraphQL
        let doc = r"
            query {
                a
                b
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        let ctx = Context::default();
        let (_, errors) = execute(doc, None, &schema, &graphql_vars! {}, &ctx)
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);

        let log = ctx.into_log();

        assert_eq!(
            &log[..2],
            ["a:start", "b:start"],
            "`query` root fields resolve concurrently, which §6.3.1 permits but does not require",
        );
    }
}

/// Root fields spread through fragments, and [`juniper::Executor::look_ahead()`] inside them.
mod fragments {
    use std::sync::Mutex;

    use juniper::{
        EmptySubscription, Executor, RootNode, ScalarValue, execute, execute_sync, graphql_object,
        graphql_vars,
    };

    #[derive(Default)]
    pub struct Context {
        resolved: Mutex<Vec<&'static str>>,
    }

    impl juniper::Context for Context {}

    impl Context {
        fn resolve(&self, field: &'static str) -> bool {
            self.resolved.lock().unwrap().push(field);
            true
        }

        fn into_resolved(self) -> Vec<&'static str> {
            self.resolved.into_inner().unwrap()
        }
    }

    pub struct Query;

    #[graphql_object(context = Context)]
    impl Query {
        fn some() -> bool {
            true
        }
    }

    pub struct Mutation;

    #[graphql_object(context = Context)]
    impl Mutation {
        fn a(context: &Context) -> bool {
            context.resolve("a")
        }

        fn b(context: &Context) -> bool {
            context.resolve("b")
        }

        fn c<__S: ScalarValue>(executor: &Executor<'_, '_, Context, __S>) -> bool {
            let lh = executor.look_ahead();

            assert_eq!(lh.field_original_name(), "c");
            assert_eq!(lh.field_alias(), Some("aliased"));

            executor.context().resolve("c")
        }
    }

    type Schema = RootNode<Query, Mutation, EmptySubscription<Context>>;

    #[tokio::test]
    async fn fragment_spread_executes_in_document_order() {
        // language=GraphQL
        let doc = r"
            mutation {
                a
                ...F
            }

            fragment F on Mutation {
                b
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        let ctx = Context::default();
        let (_, errors) = execute_sync(doc, None, &schema, &graphql_vars! {}, &ctx).unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(ctx.into_resolved(), ["a", "b"], "sync execution order");

        let ctx = Context::default();
        let (_, errors) = execute(doc, None, &schema, &graphql_vars! {}, &ctx)
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(ctx.into_resolved(), ["a", "b"], "async execution order");
    }

    #[tokio::test]
    async fn typed_inline_fragment_executes_in_document_order() {
        // language=GraphQL
        let doc = r"
            mutation {
                a
                ... on Mutation {
                    b
                }
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        let ctx = Context::default();
        let (_, errors) = execute_sync(doc, None, &schema, &graphql_vars! {}, &ctx).unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(ctx.into_resolved(), ["a", "b"], "sync execution order");

        let ctx = Context::default();
        let (_, errors) = execute(doc, None, &schema, &graphql_vars! {}, &ctx)
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(ctx.into_resolved(), ["a", "b"], "async execution order");
    }

    #[tokio::test]
    async fn look_ahead_sees_field_inside_untyped_inline_fragment() {
        // language=GraphQL
        let doc = r"
            mutation {
                ... {
                    aliased: c
                }
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        let ctx = Context::default();
        let (_, errors) = execute_sync(doc, None, &schema, &graphql_vars! {}, &ctx).unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(ctx.into_resolved(), ["c"], "sync execution");

        let ctx = Context::default();
        let (_, errors) = execute(doc, None, &schema, &graphql_vars! {}, &ctx)
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(ctx.into_resolved(), ["c"], "async execution");
    }

    #[tokio::test]
    async fn look_ahead_sees_field_inside_typed_inline_fragment() {
        // language=GraphQL
        let doc = r"
            mutation {
                ... on Mutation {
                    aliased: c
                }
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        let ctx = Context::default();
        let (_, errors) = execute_sync(doc, None, &schema, &graphql_vars! {}, &ctx).unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(ctx.into_resolved(), ["c"], "sync execution");

        let ctx = Context::default();
        let (_, errors) = execute(doc, None, &schema, &graphql_vars! {}, &ctx)
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(ctx.into_resolved(), ["c"], "async execution");
    }

    #[tokio::test]
    async fn look_ahead_sees_field_spread_through_fragment() {
        // language=GraphQL
        let doc = r"
            mutation {
                ...F
            }

            fragment F on Mutation {
                aliased: c
            }
        ";

        let schema = Schema::new(Query, Mutation, EmptySubscription::new());

        let ctx = Context::default();
        let (_, errors) = execute_sync(doc, None, &schema, &graphql_vars! {}, &ctx).unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(ctx.into_resolved(), ["c"], "sync execution");

        let ctx = Context::default();
        let (_, errors) = execute(doc, None, &schema, &graphql_vars! {}, &ctx)
            .await
            .unwrap();

        assert_eq!(errors.len(), 0);
        assert_eq!(ctx.into_resolved(), ["c"], "async execution");
    }
}

/// Mutation roots behind `Box`, `Arc`, and `&` pointers.
///
/// The pointer types forward `resolve_async()` to their target. They do not forward
/// `resolve_field_async()` or `concrete_type_name()`.
mod pointer_roots {
    use std::sync::{Arc, Mutex};

    use juniper::{
        DefaultScalarValue, EmptySubscription, GraphQLTypeAsync, RootNode, execute, graphql_object,
        graphql_value, graphql_vars,
    };

    #[derive(Default)]
    pub struct Context {
        log: Mutex<Vec<&'static str>>,
    }

    impl juniper::Context for Context {}

    impl Context {
        async fn resolve(&self, start: &'static str, end: &'static str) -> bool {
            self.log.lock().unwrap().push(start);
            tokio::task::yield_now().await;
            self.log.lock().unwrap().push(end);
            true
        }
    }

    pub struct Query;

    #[graphql_object(context = Context)]
    impl Query {
        fn some() -> bool {
            true
        }
    }

    pub struct Mutation;

    #[graphql_object(context = Context)]
    impl Mutation {
        async fn a(context: &Context) -> bool {
            context.resolve("a:start", "a:end").await
        }

        async fn b(context: &Context) -> bool {
            context.resolve("b:start", "b:end").await
        }

        async fn c(context: &Context) -> bool {
            context.resolve("c:start", "c:end").await
        }
    }

    /// Executes a mutation with `__typename`, a fragment spread, and an inline fragment against the
    /// `mutation` root.
    async fn execute_mutation<M>(mutation: M)
    where
        M: GraphQLTypeAsync<DefaultScalarValue, Context = Context, TypeInfo = ()>,
    {
        // language=GraphQL
        let doc = r"
            mutation {
                __typename
                a
                ...B
                ... on Mutation {
                    c
                }
            }

            fragment B on Mutation {
                b
            }
        ";

        let schema = RootNode::new(Query, mutation, EmptySubscription::new());
        let ctx = Context::default();

        assert_eq!(
            execute(doc, None, &schema, &graphql_vars! {}, &ctx).await,
            Ok((
                graphql_value!({"__typename": "Mutation", "a": true, "b": true, "c": true}),
                vec![],
            )),
        );
        assert_eq!(
            ctx.log.into_inner().unwrap(),
            ["a:start", "a:end", "b:start", "b:end", "c:start", "c:end"],
        );
    }

    #[tokio::test]
    async fn box_root() {
        execute_mutation(Box::new(Mutation)).await;
    }

    #[tokio::test]
    async fn arc_root() {
        execute_mutation(Arc::new(Mutation)).await;
    }

    #[tokio::test]
    async fn reference_root() {
        static MUTATION: Mutation = Mutation;

        execute_mutation(&MUTATION).await;
    }
}
