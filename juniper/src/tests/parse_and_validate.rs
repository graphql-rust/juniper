//! Tests of [`parse_and_validate()`] and [`ValidatedOperation`].
//!
//! [`ValidatedOperation`]: crate::ValidatedOperation

use std::pin::Pin;

use futures::stream;

use crate::{
    Context, DefaultScalarValue, GraphQLError, RootNode, graphql_object, graphql_subscription,
    parse_and_validate,
};

struct MyContext;

impl Context for MyContext {}

struct Query;

#[graphql_object(context = MyContext)]
impl Query {
    fn ping() -> bool {
        true
    }

    fn echo(value: i32) -> i32 {
        value
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

/// Returns the error of [`parse_and_validate()`] for the `operation_name` operation of the
/// `query`.
fn error(schema: &Schema, query: &str, operation_name: Option<&str>) -> GraphQLError {
    parse_and_validate(query, operation_name, schema).expect_err("invalid operation")
}

mod operation_type {
    use crate::{ast::OperationType, parse_and_validate};

    use super::schema;

    /// Returns the [`OperationType`] of the `operation_name` operation of the `query`.
    fn operation_type(query: &str, operation_name: Option<&str>) -> OperationType {
        parse_and_validate(query, operation_name, &schema())
            .expect("valid operation")
            .operation_type()
    }

    #[test]
    fn query() {
        assert_eq!(operation_type("{ ping }", None), OperationType::Query);
    }

    #[test]
    fn mutation() {
        assert_eq!(
            operation_type("mutation { pong }", None),
            OperationType::Mutation,
        );
    }

    #[test]
    fn subscription() {
        assert_eq!(
            operation_type("subscription { tick }", None),
            OperationType::Subscription,
        );
    }

    #[test]
    fn named_operation() {
        let query = "query A { ping } mutation B { pong }";

        assert_eq!(operation_type(query, Some("A")), OperationType::Query);
        assert_eq!(operation_type(query, Some("B")), OperationType::Mutation);
    }
}

mod rejects {
    use crate::{GraphQLError, parser::ParseError};

    use super::{error, schema};

    #[test]
    fn multiple_operations_without_name() {
        let err = error(&schema(), "query A { ping } mutation B { pong }", None);

        assert_eq!(err, GraphQLError::MultipleOperationsProvided);
    }

    #[test]
    fn unknown_operation_name() {
        let err = error(&schema(), "query A { ping }", Some("B"));

        assert_eq!(err, GraphQLError::UnknownOperationName);
    }

    #[test]
    fn invalid_syntax() {
        let err = error(&schema(), "{ ping ", None);

        assert!(matches!(err, GraphQLError::ParseError(_)), "got {err:?}");
    }

    #[test]
    fn invalid_document() {
        let err = error(&schema(), "{ unknownField }", None);

        assert!(
            matches!(err, GraphQLError::ValidationError(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn document_over_limit() {
        let err = error(&schema().limit_fields(1), "{ a: ping b: ping }", None);

        assert!(
            matches!(&err, GraphQLError::ParseError(e) if e.item == ParseError::FieldLimitExceeded(1)),
            "got {err:?}",
        );
    }

    #[test]
    fn introspection_if_disabled() {
        let schema = schema().disable_introspection();

        let err = error(&schema, "{ __schema { queryType { name } } }", None);

        assert!(
            matches!(err, GraphQLError::ValidationError(_)),
            "got {err:?}"
        );
    }
}

mod execute {
    use crate::{GraphQLError, graphql, parse_and_validate};

    use super::{MyContext, schema};

    /// Query with a required variable.
    const ECHO: &str = "query Echo($value: Int!) { echo(value: $value) }";

    /// Query with a variable that has a default value.
    const ECHO_WITH_DEFAULT: &str = "query Echo($value: Int = 3) { echo(value: $value) }";

    #[test]
    fn synchronously() {
        let schema = schema();
        let operation = parse_and_validate("{ ping }", None, &schema).unwrap();

        let result = operation.execute_sync(&graphql::vars! {}, &MyContext);

        assert_eq!(result, Ok((graphql::value!({"ping": true}), vec![])));
    }

    #[tokio::test]
    async fn asynchronously() {
        let schema = schema();
        let operation = parse_and_validate("mutation { pong }", None, &schema).unwrap();

        let result = operation.execute(&graphql::vars! {}, &MyContext).await;

        assert_eq!(result, Ok((graphql::value!({"pong": true}), vec![])));
    }

    #[test]
    fn named_operation() {
        let schema = schema();
        let query = "query A { ping } query B { echo(value: 1) }";
        let operation = parse_and_validate(query, Some("B"), &schema).unwrap();

        let result = operation.execute_sync(&graphql::vars! {}, &MyContext);

        assert_eq!(result, Ok((graphql::value!({"echo": 1}), vec![])));
    }

    #[test]
    fn fragments() {
        let schema = schema();
        let query =
            "query A { ...Echo } query B { ping } fragment Echo on Query { echo(value: 1) }";
        let operation = parse_and_validate(query, Some("A"), &schema).unwrap();

        let result = operation.execute_sync(&graphql::vars! {}, &MyContext);

        assert_eq!(result, Ok((graphql::value!({"echo": 1}), vec![])));
    }

    #[test]
    fn default_values() {
        let schema = schema();
        let operation = parse_and_validate(ECHO_WITH_DEFAULT, None, &schema).unwrap();

        let result = operation.execute_sync(&graphql::vars! {}, &MyContext);

        assert_eq!(result, Ok((graphql::value!({"echo": 3}), vec![])));
    }

    #[test]
    fn provided_variables_over_default_values() {
        let schema = schema();
        let operation = parse_and_validate(ECHO_WITH_DEFAULT, None, &schema).unwrap();

        let result = operation.execute_sync(&graphql::vars! {"value": 5}, &MyContext);

        assert_eq!(result, Ok((graphql::value!({"echo": 5}), vec![])));
    }

    #[test]
    fn more_than_once() {
        let schema = schema();
        let operation = parse_and_validate(ECHO, None, &schema).unwrap();

        for value in [1, 2] {
            let result = operation.execute_sync(&graphql::vars! {"value": (value)}, &MyContext);

            assert_eq!(result, Ok((graphql::value!({"echo": value}), vec![])));
        }
    }

    #[test]
    fn rejects_invalid_variables() {
        let schema = schema();
        let operation = parse_and_validate(ECHO, None, &schema).unwrap();

        for variables in [graphql::vars! {}, graphql::vars! {"value": "one"}] {
            let result = operation.execute_sync(&variables, &MyContext);

            assert!(
                matches!(result, Err(GraphQLError::ValidationError(_))),
                "got {result:?}",
            );
        }
    }

    #[test]
    fn rejects_subscription() {
        let schema = schema();
        let operation = parse_and_validate("subscription { tick }", None, &schema).unwrap();

        let result = operation.execute_sync(&graphql::vars! {}, &MyContext);

        assert_eq!(result, Err(GraphQLError::IsSubscription));
    }
}

mod resolve_into_stream {
    use futures::StreamExt as _;

    use crate::{GraphQLError, Value, graphql, parse_and_validate};

    use super::{MyContext, schema};

    #[tokio::test]
    async fn subscription() {
        let schema = schema();
        let operation = parse_and_validate("subscription { tick }", None, &schema).unwrap();

        let (value, errors) = operation
            .resolve_into_stream(&graphql::vars! {}, &MyContext)
            .await
            .expect("subscription should resolve");

        assert!(errors.is_empty(), "got {errors:?}");
        let Value::Object(fields) = value else {
            panic!("subscription response value should be an object");
        };
        let mut fields = fields.into_iter();
        let Some((name, Value::Scalar(stream))) = fields.next() else {
            panic!("subscription field should resolve to a stream");
        };
        assert_eq!(name, "tick");
        assert!(
            fields.next().is_none(),
            "subscription should have one field"
        );
        assert_eq!(
            stream.collect::<Vec<_>>().await,
            [Ok(graphql::value!(true))],
        );
    }

    #[tokio::test]
    async fn rejects_query() {
        let schema = schema();
        let operation = parse_and_validate("{ ping }", None, &schema).unwrap();

        let result = operation
            .resolve_into_stream(&graphql::vars! {}, &MyContext)
            .await;

        assert!(
            matches!(result, Err(GraphQLError::NotSubscription)),
            "expected `NotSubscription`",
        );
    }
}
