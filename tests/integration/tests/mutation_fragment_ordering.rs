//! Checks that root fields of a `mutation` operation execute in document order when
//! spread through fragments, per [§6.3.1] of the GraphQL spec.
//! See [#1398](https://github.com/graphql-rust/juniper/issues/1398) for details.
//!
//! [§6.3.1]: https://spec.graphql.org/September2025/#sec-Normal-and-Serial-Execution

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
