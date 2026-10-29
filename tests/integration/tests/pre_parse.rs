//! Tests of operations that are parsed and validated before execution.

use std::pin::Pin;

use futures::{Stream, StreamExt as _, TryFutureExt as _};
use juniper::{
    EmptyMutation, FieldError, OperationType, RootNode, ValidatedOperation, graphql_object,
    graphql_subscription, graphql_vars,
};

pub struct Context;

impl juniper::Context for Context {}

pub type UserStream = Pin<Box<dyn Stream<Item = Result<User, FieldError>> + Send>>;

pub struct Query;

#[graphql_object(context = Context)]
impl Query {
    fn users() -> Vec<User> {
        vec![User]
    }
}

pub struct Subscription;

#[graphql_subscription(context = Context)]
impl Subscription {
    async fn users() -> UserStream {
        Box::pin(futures::stream::iter(vec![Ok(User)]))
    }
}

#[derive(Clone)]
pub struct User;

#[graphql_object(context = Context)]
impl User {
    fn id() -> i32 {
        1
    }
}

type Schema = RootNode<Query, EmptyMutation<Context>, Subscription>;

fn schema() -> Schema {
    Schema::new(Query, EmptyMutation::new(), Subscription)
}

#[tokio::test]
async fn executes_query() {
    let schema = schema();
    let operation = ValidatedOperation::new("query { users { id } }", None, &schema).unwrap();
    assert_eq!(operation.operation_type(), OperationType::Query);

    let (_, errors) = operation
        .execute(&graphql_vars! {}, &Context)
        .await
        .unwrap();

    assert!(errors.is_empty());
}

#[tokio::test]
async fn resolves_subscription() {
    let schema = schema();
    let operation =
        ValidatedOperation::new("subscription { users { id } }", None, &schema).unwrap();
    assert_eq!(operation.operation_type(), OperationType::Subscription);

    let mut stream = operation
        .resolve_into_stream(&graphql_vars! {}, &Context)
        .map_ok(|(stream, errors)| juniper_subscriptions::Connection::from_stream(stream, errors))
        .await
        .unwrap();

    let _ = stream.next().await.unwrap();
}
