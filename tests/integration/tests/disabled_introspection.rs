//! Checks whether [`RootNode::disable_introspection()`] works.

use futures::stream;
use juniper::{
    execute, graphql_object, graphql_subscription, graphql_vars, resolve_into_stream, GraphQLError,
    RootNode,
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
async fn query_schema() {
    // language=GraphQL
    let query = "query { __schema { queryType { name } } }";

    let schema = RootNode::new(Query, Mutation, Subscription).disable_introspection();

    match execute(query, None, &schema, &graphql_vars! {}, &()).await {
        Err(GraphQLError::ValidationError(errors)) => {
            assert_eq!(errors.len(), 1);

            let err = errors.first().unwrap();

            assert_eq!(
                err.message(),
                "GraphQL introspection is not allowed, but the operation contained `__schema`",
            );
            assert_eq!(err.locations()[0].index(), 8);
            assert_eq!(err.locations()[0].line(), 0);
            assert_eq!(err.locations()[0].column(), 8);
        }
        res => panic!("expected `ValidationError`, returned: {res:#?}"),
    }
}

#[tokio::test]
async fn mutation_schema() {
    // language=GraphQL
    let query = "mutation { __schema { queryType { name } } }";

    let schema = RootNode::new(Query, Mutation, Subscription).disable_introspection();

    match execute(query, None, &schema, &graphql_vars! {}, &()).await {
        Err(GraphQLError::ValidationError(errors)) => {
            assert_eq!(errors.len(), 2);

            let err = errors.first().unwrap();

            assert_eq!(
                err.message(),
                "GraphQL introspection is not allowed, but the operation contained `__schema`",
            );
            assert_eq!(err.locations()[0].index(), 11);
            assert_eq!(err.locations()[0].line(), 0);
            assert_eq!(err.locations()[0].column(), 11);
        }
        res => panic!("expected `ValidationError`, returned: {res:#?}"),
    }
}

#[tokio::test]
async fn subscription_schema() {
    // language=GraphQL
    let query = "subscription { __schema { queryType { name } } }";

    let schema = RootNode::new(Query, Mutation, Subscription).disable_introspection();

    match resolve_into_stream(query, None, &schema, &graphql_vars! {}, &()).await {
        Err(GraphQLError::ValidationError(errors)) => {
            assert_eq!(errors.len(), 2);

            let err = errors.first().unwrap();

            assert_eq!(
                err.message(),
                "GraphQL introspection is not allowed, but the operation contained `__schema`",
            );
            assert_eq!(err.locations()[0].index(), 15);
            assert_eq!(err.locations()[0].line(), 0);
            assert_eq!(err.locations()[0].column(), 15);
        }
        _ => panic!("expected `ValidationError`"),
    };
}

#[tokio::test]
async fn query_type() {
    // language=GraphQL
    let query = r#"query { __type(name: "String") { name } }"#;

    let schema = RootNode::new(Query, Mutation, Subscription).disable_introspection();

    match execute(query, None, &schema, &graphql_vars! {}, &()).await {
        Err(GraphQLError::ValidationError(errors)) => {
            assert_eq!(errors.len(), 1);

            let err = errors.first().unwrap();

            assert_eq!(
                err.message(),
                "GraphQL introspection is not allowed, but the operation contained `__type`",
            );
            assert_eq!(err.locations()[0].index(), 8);
            assert_eq!(err.locations()[0].line(), 0);
            assert_eq!(err.locations()[0].column(), 8);
        }
        res => panic!("expected `ValidationError`, returned: {res:#?}"),
    }
}

#[tokio::test]
async fn mutation_type() {
    // language=GraphQL
    let query = r#"mutation { __type(name: "String") { name } }"#;

    let schema = RootNode::new(Query, Mutation, Subscription).disable_introspection();

    match execute(query, None, &schema, &graphql_vars! {}, &()).await {
        Err(GraphQLError::ValidationError(errors)) => {
            assert_eq!(errors.len(), 2);

            let err = errors.first().unwrap();

            assert_eq!(
                err.message(),
                "GraphQL introspection is not allowed, but the operation contained `__type`",
            );
            assert_eq!(err.locations()[0].index(), 11);
            assert_eq!(err.locations()[0].line(), 0);
            assert_eq!(err.locations()[0].column(), 11);
        }
        res => panic!("expected `ValidationError`, returned: {res:#?}"),
    }
}

#[tokio::test]
async fn subscription_type() {
    // language=GraphQL
    let query = r#"subscription Subscription { __type(name: "String") { name } }"#;

    let schema = RootNode::new(Query, Mutation, Subscription).disable_introspection();

    match resolve_into_stream(query, None, &schema, &graphql_vars! {}, &()).await {
        Err(GraphQLError::ValidationError(errors)) => {
            assert_eq!(errors.len(), 2);

            let err = errors.first().unwrap();

            assert_eq!(
                err.message(),
                "GraphQL introspection is not allowed, but the operation contained `__type`",
            );
            assert_eq!(err.locations()[0].index(), 28);
            assert_eq!(err.locations()[0].line(), 0);
            assert_eq!(err.locations()[0].column(), 28);
        }
        _ => panic!("expected `ValidationError`"),
    };
}
