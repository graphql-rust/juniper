//! Regression tests for [`GraphQLError::NotSupported`].
//!
//! Executing an operation whose root type the schema doesn't define — e.g. a
//! `mutation` against [`EmptyMutation`], or a `subscription` against
//! [`EmptySubscription`] — must return a request error, not panic.

use crate::{
    GraphQLError,
    ast::OperationType,
    graphql,
    schema::model::RootNode,
    tests::fixtures::starwars::schema::{Database, Query},
    types::scalars::{EmptyMutation, EmptySubscription},
};

fn query_only_schema() -> RootNode<Query, EmptyMutation<Database>, EmptySubscription<Database>> {
    RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    )
}

#[test]
fn sync_mutation_on_query_only_schema_errors_instead_of_panicking() {
    let schema = query_only_schema();
    let database = Database::new();
    let result = crate::execute_sync(
        "mutation { foo }",
        None,
        &schema,
        &graphql::vars! {},
        &database,
    );
    assert!(
        matches!(
            &result,
            Err(GraphQLError::NotSupported(OperationType::Mutation)),
        ),
        "expected NotSupported(Mutation), got {result:?}",
    );
}

#[tokio::test]
async fn async_mutation_on_query_only_schema_errors_instead_of_panicking() {
    let schema = query_only_schema();
    let database = Database::new();
    let result = crate::execute(
        "mutation { foo }",
        None,
        &schema,
        &graphql::vars! {},
        &database,
    )
    .await;
    assert!(
        matches!(
            &result,
            Err(GraphQLError::NotSupported(OperationType::Mutation)),
        ),
        "expected NotSupported(Mutation), got {result:?}",
    );
}

#[tokio::test]
async fn subscription_without_subscription_root_errors_instead_of_panicking() {
    let schema = query_only_schema();
    let database = Database::new();
    let result = crate::resolve_into_stream(
        "subscription { foo }",
        None,
        &schema,
        &graphql::vars! {},
        &database,
    )
    .await;
    match result {
        Err(GraphQLError::NotSupported(OperationType::Subscription)) => {}
        Err(other) => panic!("expected NotSupported(Subscription), got {other:?}"),
        Ok(_) => panic!("expected an error, got a stream"),
    }
}
