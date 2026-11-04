//! Order of the fields of an object with pending fields during asynchronous execution.

use crate::{EmptyMutation, EmptySubscription, RootNode, graphql, graphql_object};

struct Query;

#[graphql_object]
impl Query {
    async fn pending() -> i32 {
        tokio::task::yield_now().await;
        1
    }

    fn ready() -> i32 {
        2
    }
}

async fn response_keys(document: &str) -> Vec<String> {
    let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
    let (value, errors) = crate::execute(document, None, &schema, &graphql::vars! {}, &())
        .await
        .unwrap();
    assert_eq!(errors, []);
    value
        .as_object_value()
        .unwrap()
        .iter()
        .map(|(key, _)| key.to_string())
        .collect()
}

#[tokio::test]
async fn keeps_resolved_fields_behind_pending_field() {
    let keys = response_keys("{ pending ready __typename }").await;

    assert_eq!(keys, ["pending", "ready", "__typename"]);
}

#[tokio::test]
async fn keeps_resolved_fields_before_pending_field() {
    let keys = response_keys("{ ready __typename pending }").await;

    assert_eq!(keys, ["ready", "__typename", "pending"]);
}
