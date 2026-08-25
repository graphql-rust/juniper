//! Tests for [`parse_and_validate()`].
//!
//! [`parse_and_validate()`] should run the same parsing and validation as
//! [`execute()`]/[`execute_sync()`], report the same errors, and report the
//! [`OperationType`] of the operation that would be executed.
//!
//! [`execute()`]: crate::execute
//! [`execute_sync()`]: crate::execute_sync
//! [`OperationType`]: crate::ast::OperationType
//! [`parse_and_validate()`]: crate::parse_and_validate

use std::pin::Pin;

use futures::stream;

use crate::{
    Context, DefaultScalarValue, GraphQLError, RootNode, ast::OperationType, graphql,
    graphql_object, graphql_subscription,
};

struct MyContext;
impl Context for MyContext {}

struct Query;

#[graphql_object(context = MyContext)]
impl Query {
    fn ping() -> bool {
        true
    }
}

struct Mutation;

#[graphql_object(context = MyContext)]
impl Mutation {
    fn pong() -> bool {
        true
    }
}

type BoolStream = Pin<Box<dyn futures::Stream<Item = bool> + Send>>;

struct Subscription;

#[graphql_subscription(context = MyContext)]
impl Subscription {
    async fn tick() -> BoolStream {
        Box::pin(stream::once(async { true }))
    }
}

type Schema = RootNode<Query, Mutation, Subscription, DefaultScalarValue>;

fn schema() -> Schema {
    RootNode::new(Query, Mutation, Subscription)
}

#[test]
fn reports_query_operation_type() {
    let result = crate::parse_and_validate("{ ping }", None, &schema(), &graphql::vars! {});
    assert_eq!(result.unwrap(), OperationType::Query);
}

#[test]
fn reports_mutation_operation_type() {
    let result =
        crate::parse_and_validate("mutation { pong }", None, &schema(), &graphql::vars! {});
    assert_eq!(result.unwrap(), OperationType::Mutation);
}

#[test]
fn reports_subscription_operation_type() {
    let result =
        crate::parse_and_validate("subscription { tick }", None, &schema(), &graphql::vars! {});
    assert_eq!(result.unwrap(), OperationType::Subscription);
}

#[test]
fn selects_named_operation() {
    let query = "query A { ping } mutation B { pong }";
    let result = crate::parse_and_validate(query, Some("B"), &schema(), &graphql::vars! {});
    assert_eq!(result.unwrap(), OperationType::Mutation);
}

#[test]
fn multiple_operations_without_a_name_errors() {
    let query = "query A { ping } mutation B { pong }";
    let result = crate::parse_and_validate(query, None, &schema(), &graphql::vars! {});
    assert!(
        matches!(result, Err(GraphQLError::MultipleOperationsProvided)),
        "got {result:?}",
    );
}

#[test]
fn validation_errors_are_reported() {
    let result = crate::parse_and_validate("{ unknownField }", None, &schema(), &graphql::vars! {});
    assert!(
        matches!(result, Err(GraphQLError::ValidationError(_))),
        "got {result:?}",
    );
}

#[test]
fn parse_errors_are_reported() {
    let result = crate::parse_and_validate("{ ping ", None, &schema(), &graphql::vars! {});
    assert!(
        matches!(result, Err(GraphQLError::ParseError(_))),
        "got {result:?}",
    );
}
