//! Checks that `__typename` field queries okay (and not okay) on root types.
//! See [#372](https://github.com/graphql-rust/juniper/issues/372) for details.

use futures::stream;
use juniper::{
    execute, graphql_object, graphql_subscription, graphql_value, graphql_vars,
    resolve_into_stream, GraphQLError, RootNode,
};

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

    match resolve_into_stream(query, None, &schema, &graphql_vars! {}, &()).await {
        Err(GraphQLError::ValidationError(mut errors)) => {
            assert_eq!(errors.len(), 1);

            let err = errors.pop().unwrap();

            assert_eq!(
                err.message(),
                "`__typename` may not be included as a root field in a \
                 subscription operation",
            );
            assert_eq!(err.locations()[0].index(), 15);
            assert_eq!(err.locations()[0].line(), 0);
            assert_eq!(err.locations()[0].column(), 15);
        }
        _ => panic!("Expected ValidationError"),
    };
}

#[tokio::test]
async fn explicit_subscription_typename() {
    let query = r#"subscription Subscription { __typename }"#;

    let schema = RootNode::new(Query, Mutation, Subscription);

    match resolve_into_stream(query, None, &schema, &graphql_vars! {}, &()).await {
        Err(GraphQLError::ValidationError(mut errors)) => {
            assert_eq!(errors.len(), 1);

            let err = errors.pop().unwrap();

            assert_eq!(
                err.message(),
                "`__typename` may not be included as a root field in a \
                 subscription operation"
            );
            assert_eq!(err.locations()[0].index(), 28);
            assert_eq!(err.locations()[0].line(), 0);
            assert_eq!(err.locations()[0].column(), 28);
        }
        _ => panic!("Expected ValidationError"),
    };
}
