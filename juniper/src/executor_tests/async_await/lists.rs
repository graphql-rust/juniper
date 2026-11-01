//! Resolution of lists with pending and ready items during asynchronous execution.

use crate::{
    EmptyMutation, EmptySubscription, FieldError, FieldResult, RootNode, graphql, graphql_object,
};

/// List item. The `value` of an item with an even number is pending once before it resolves.
struct Item(i32);

#[graphql_object]
impl Item {
    async fn value(&self, fail: Option<i32>) -> FieldResult<i32> {
        if self.0 % 2 == 0 {
            tokio::task::yield_now().await;
        }
        if fail == Some(self.0) {
            return Err(FieldError::from("failed"));
        }
        Ok(self.0)
    }
}

struct Query;

#[graphql_object]
impl Query {
    async fn items(count: i32) -> Vec<Item> {
        (0..count).map(Item).collect()
    }
}

#[tokio::test]
async fn keeps_order_of_pending_and_ready_items() {
    let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

    let (value, errors) = crate::execute(
        "{ items(count: 5) { value } }",
        None,
        &schema,
        &graphql::vars! {},
        &(),
    )
    .await
    .unwrap();

    assert_eq!(errors, []);
    assert_eq!(
        value,
        graphql::value!({"items": [
            {"value": 0},
            {"value": 1},
            {"value": 2},
            {"value": 3},
            {"value": 4},
        ]}),
    );
}

mod null_item {
    use super::Query;
    use crate::{EmptyMutation, EmptySubscription, RootNode, Value, graphql};

    async fn execute_failing_at(item: i32) -> (Value, Vec<String>) {
        let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        let document = format!("{{ items(count: 5) {{ value(fail: {item}) }} }}");
        let (value, errors) = crate::execute(&document, None, &schema, &graphql::vars! {}, &())
            .await
            .unwrap();
        let errors = errors
            .iter()
            .map(|e| format!("{}: {}", e.path().join("."), e.error().message()))
            .collect();
        (value, errors)
    }

    #[tokio::test]
    async fn from_ready_item_makes_list_null() {
        let (value, errors) = execute_failing_at(1).await;

        assert_eq!(value, graphql::value!(null));
        assert_eq!(errors, ["items.value: failed"]);
    }

    #[tokio::test]
    async fn from_pending_item_makes_list_null() {
        let (value, errors) = execute_failing_at(2).await;

        assert_eq!(value, graphql::value!(null));
        assert_eq!(errors, ["items.value: failed"]);
    }
}
