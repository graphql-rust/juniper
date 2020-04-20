#![deny(warnings)]

use futures::{Stream, StreamExt};
use juniper::http::GraphQLRequest;
use juniper::{DefaultScalarValue, EmptyMutation, FieldError, RootNode, SubscriptionCoordinator};
use juniper_subscriptions::Coordinator;
use std::pin::Pin;

#[derive(Clone)]
pub struct Database;

impl juniper::Context for Database {}

impl Database {
    fn new() -> Self {
        Self {}
    }
}

pub struct Query;

#[juniper::graphql_object(Context = Database)]
impl Query {
    fn hello_world() -> &str {
        "Hello World!"
    }
}

pub struct Subscription;

type StringStream = Pin<Box<dyn Stream<Item = Result<String, FieldError>> + Send>>;

#[juniper::graphql_subscription(Context = Database)]
impl Subscription {
    async fn hello_world() -> StringStream {
        let stream =
            tokio::stream::iter(vec![Ok(String::from("Hello")), Ok(String::from("World!"))]);
        Box::pin(stream)
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<Database>, Subscription>;

fn schema() -> Schema {
    Schema::new(Query {}, EmptyMutation::new(), Subscription {})
}

#[tokio::main]
async fn main() {
    let schema = schema();
    let coordinator = Coordinator::new(schema);
    let req: GraphQLRequest<DefaultScalarValue> = serde_json::from_str(
        r#"
        {
            "query": "subscription { helloWorld }"
        }
    "#,
    )
        .unwrap();
    let ctx = Database::new();
    let mut conn = coordinator.subscribe(&req, &ctx).await.unwrap();
    while let Some(result) = conn.next().await {
        println!("{}", serde_json::to_string(&result).unwrap());
    }
}
