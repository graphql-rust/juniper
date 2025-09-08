//! This example demonstrates how to use a [`Coordinator`] for executing a GraphQL subscription.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use juniper::{
    DefaultScalarValue, EmptyMutation, FieldError, RootNode, SubscriptionCoordinator,
    graphql_object, graphql_subscription, http::GraphQLRequest,
};
use juniper_subscriptions::Coordinator;

#[derive(Clone)]
struct Database;

impl juniper::Context for Database {}

impl Database {
    fn new() -> Self {
        Self
    }
}

struct Query;

#[graphql_object(context = Database)]
impl Query {
    fn hello_world() -> &'static str {
        "Hello World!"
    }
}

struct Subscription;

type StringStream = Pin<Box<dyn Stream<Item = Result<String, FieldError>> + Send>>;

#[graphql_subscription(context = Database)]
impl Subscription {
    async fn hello_world() -> StringStream {
        let stream =
            futures::stream::iter(vec![Ok(String::from("Hello")), Ok(String::from("World!"))]);
        Box::pin(stream)
    }
}

type Schema = RootNode<Query, EmptyMutation<Database>, Subscription>;

fn schema() -> Schema {
    Schema::new(Query, EmptyMutation::new(), Subscription)
}

#[tokio::main]
async fn main() {
    let schema = schema();
    let coordinator = Coordinator::new(schema);
    let req: GraphQLRequest<DefaultScalarValue> = serde_json::from_str(
        r#"{
            "query": "subscription { helloWorld }"
        }"#,
    )
    .unwrap();
    let ctx = Database::new();
    let mut conn = coordinator.subscribe(&req, &ctx).await.unwrap();
    while let Some(result) = conn.next().await {
        println!("{}", serde_json::to_string(&result).unwrap());
    }
}
