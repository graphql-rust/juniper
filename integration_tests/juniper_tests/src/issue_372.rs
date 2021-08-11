//! Checks that `__typename` field queries okay on root types.
//! See [#372](https://github.com/graphql-rust/juniper/issues/372) for details.

use futures::{stream, StreamExt as _};
use juniper::{graphql_object, graphql_subscription, graphql_value, RootNode, Value, Variables};

pub struct Query;

#[graphql_object]
impl Query {
    fn some() -> bool {
        true
    }
}

pub struct Mutation;

#[graphql_object]
impl Mutation {
    fn another() -> bool {
        false
    }
}

pub struct Subscription;

#[graphql_subscription]
impl Subscription {
    async fn another() -> stream::Empty<bool> {
        stream::empty()
    }
}

#[tokio::test]
async fn implicit_query_typename() {
    let query = r#"{ __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);
    let (res, errors) = juniper::execute(query, None, &schema, &Variables::new(), &())
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(res, graphql_value!({"__typename": "Query"}));
}

#[tokio::test]
async fn query_typename() {
    let query = r#"query { __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);
    let (res, errors) = juniper::execute(query, None, &schema, &Variables::new(), &())
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(res, graphql_value!({"__typename": "Query"}));
}

#[tokio::test]
async fn explicit_query_typename() {
    let query = r#"query Query { __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);
    let (res, errors) = juniper::execute(query, None, &schema, &Variables::new(), &())
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(res, graphql_value!({"__typename": "Query"}));
}

#[tokio::test]
async fn mutation_typename() {
    let query = r#"mutation { __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);
    let (res, errors) = juniper::execute(query, None, &schema, &Variables::new(), &())
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(res, graphql_value!({"__typename": "Mutation"}));
}

#[tokio::test]
async fn explicit_mutation_typename() {
    let query = r#"mutation Mutation { __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);
    let (res, errors) = juniper::execute(query, None, &schema, &Variables::new(), &())
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(res, graphql_value!({"__typename": "Mutation"}));
}

#[tokio::test]
async fn subscription_typename() {
    let query = r#"subscription { __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);
    let (res, errors) = juniper::resolve_into_stream(query, None, &schema, &Variables::new(), &())
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
    assert!(matches!(res, Value::Object(_)));
    if let Value::Object(mut obj) = res {
        assert!(obj.contains_field("__typename"));

        let val = obj.get_mut_field_value("__typename").unwrap();
        assert!(matches!(val, Value::Scalar(_)));
        if let Value::Scalar(ref mut stream) = val {
            assert_eq!(
                stream.next().await,
                Some(Ok(graphql_value!("Subscription"))),
            );
        }
    }
}

#[tokio::test]
async fn explicit_subscription_typename() {
    let query = r#"subscription Subscription { __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);
    let (res, errors) = juniper::resolve_into_stream(query, None, &schema, &Variables::new(), &())
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
    assert!(matches!(res, Value::Object(_)));
    if let Value::Object(mut obj) = res {
        assert!(obj.contains_field("__typename"));

        let val = obj.get_mut_field_value("__typename").unwrap();
        assert!(matches!(val, Value::Scalar(_)));
        if let Value::Scalar(ref mut stream) = val {
            assert_eq!(
                stream.next().await,
                Some(Ok(graphql_value!("Subscription"))),
            );
        }
    }
}
