//! Checks that `__typename` field queries okay on root types.
//! See [#372](https://github.com/graphql-rust/juniper/issues/372) for details.

use futures::{stream, FutureExt as _};
use juniper::{
    execute, graphql_object, graphql_subscription, graphql_value, graphql_vars,
    resolve_into_stream, RootNode,
};

use crate::util::extract_next;

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

    assert_eq!(
        execute(query, None, &schema, &graphql_vars! {}, &()).await,
        Ok((graphql_value!({"__typename": "Query"}), vec![])),
    );
}

#[tokio::test]
async fn query_typename() {
    let query = r#"query { __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);

    assert_eq!(
        execute(query, None, &schema, &graphql_vars! {}, &()).await,
        Ok((graphql_value!({"__typename": "Query"}), vec![])),
    );
}

#[tokio::test]
async fn explicit_query_typename() {
    let query = r#"query Query { __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);

    assert_eq!(
        execute(query, None, &schema, &graphql_vars! {}, &()).await,
        Ok((graphql_value!({"__typename": "Query"}), vec![])),
    );
}

#[tokio::test]
async fn mutation_typename() {
    let query = r#"mutation { __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);

    assert_eq!(
        execute(query, None, &schema, &graphql_vars! {}, &()).await,
        Ok((graphql_value!({"__typename": "Mutation"}), vec![])),
    );
}

#[tokio::test]
async fn explicit_mutation_typename() {
    let query = r#"mutation Mutation { __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);

    assert_eq!(
        execute(query, None, &schema, &graphql_vars! {}, &()).await,
        Ok((graphql_value!({"__typename": "Mutation"}), vec![])),
    );
}

#[tokio::test]
async fn subscription_typename() {
    let query = r#"subscription { __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);

    assert_eq!(
        resolve_into_stream(query, None, &schema, &graphql_vars! {}, &())
            .then(|s| extract_next(s))
            .await,
        Ok((graphql_value!({"__typename": "Subscription"}), vec![])),
    );
}

#[tokio::test]
async fn explicit_subscription_typename() {
    let query = r#"subscription Subscription { __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);

    assert_eq!(
        resolve_into_stream(query, None, &schema, &graphql_vars! {}, &())
            .then(|s| extract_next(s))
            .await,
        Ok((graphql_value!({"__typename": "Subscription"}), vec![])),
    );
}
