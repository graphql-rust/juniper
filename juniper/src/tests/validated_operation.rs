//! Tests of [`ValidatedOperation`].
//!
//! [`ValidatedOperation`]: crate::ValidatedOperation

use std::pin::Pin;

use futures::stream;

use crate::{Context, DefaultScalarValue, RootNode, graphql_object, graphql_subscription};

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

mod operation_type {
    use crate::{ValidatedOperation, ast::OperationType};

    use super::{Mutation, Query, Schema, Subscription};

    #[test]
    fn query() {
        let schema = Schema::new(Query, Mutation, Subscription);

        let operation = ValidatedOperation::new("{ ping }", None, &schema).unwrap();

        assert_eq!(operation.operation_type(), OperationType::Query);
    }

    #[test]
    fn mutation() {
        let schema = Schema::new(Query, Mutation, Subscription);

        let operation = ValidatedOperation::new("mutation { pong }", None, &schema).unwrap();

        assert_eq!(operation.operation_type(), OperationType::Mutation);
    }

    #[test]
    fn subscription() {
        let schema = Schema::new(Query, Mutation, Subscription);

        let operation = ValidatedOperation::new("subscription { tick }", None, &schema).unwrap();

        assert_eq!(operation.operation_type(), OperationType::Subscription);
    }

    #[test]
    fn named_operation() {
        let schema = Schema::new(Query, Mutation, Subscription);
        let query = "query A { ping } mutation B { pong }";

        let a = ValidatedOperation::new(query, Some("A"), &schema).unwrap();
        let b = ValidatedOperation::new(query, Some("B"), &schema).unwrap();

        assert_eq!(a.operation_type(), OperationType::Query);
        assert_eq!(b.operation_type(), OperationType::Mutation);
    }
}

mod rejects {
    use crate::{GraphQLError, ValidatedOperation, parser::ParseError};

    use super::{Mutation, Query, Schema, Subscription};

    #[test]
    fn multiple_operations_without_name() {
        let schema = Schema::new(Query, Mutation, Subscription);
        let query = "query A { ping } mutation B { pong }";

        let err = ValidatedOperation::new(query, None, &schema).unwrap_err();

        assert_eq!(err, GraphQLError::MultipleOperationsProvided);
    }

    #[test]
    fn unknown_operation_name() {
        let schema = Schema::new(Query, Mutation, Subscription);

        let err = ValidatedOperation::new("query A { ping }", Some("B"), &schema).unwrap_err();

        assert_eq!(err, GraphQLError::UnknownOperationName);
    }

    #[test]
    fn invalid_syntax() {
        let schema = Schema::new(Query, Mutation, Subscription);

        let err = ValidatedOperation::new("{ ping ", None, &schema).unwrap_err();

        assert!(matches!(err, GraphQLError::ParseError(_)), "got {err:?}");
    }

    #[test]
    fn invalid_document() {
        let schema = Schema::new(Query, Mutation, Subscription);

        let err = ValidatedOperation::new("{ unknownField }", None, &schema).unwrap_err();

        assert!(
            matches!(err, GraphQLError::ValidationError(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn document_over_limit() {
        let schema = Schema::new(Query, Mutation, Subscription).limit_fields(1);

        let err = ValidatedOperation::new("{ a: ping b: ping }", None, &schema).unwrap_err();

        assert!(
            matches!(&err, GraphQLError::ParseError(e) if e.item == ParseError::FieldLimitExceeded(1)),
            "got {err:?}",
        );
    }

    #[test]
    fn introspection_if_disabled() {
        let schema = Schema::new(Query, Mutation, Subscription).disable_introspection();
        let query = "{ __schema { queryType { name } } }";

        let err = ValidatedOperation::new(query, None, &schema).unwrap_err();

        assert!(
            matches!(err, GraphQLError::ValidationError(_)),
            "got {err:?}"
        );
    }
}

mod execute {
    use crate::{GraphQLError, ValidatedOperation, graphql};

    use super::{Mutation, MyContext, Query, Schema, Subscription};

    /// Query with a required variable.
    const ECHO: &str = "query Echo($value: Int!) { echo(value: $value) }";

    /// Query with a variable that has a default value.
    const ECHO_WITH_DEFAULT: &str = "query Echo($value: Int = 3) { echo(value: $value) }";

    #[test]
    fn synchronously() {
        let schema = Schema::new(Query, Mutation, Subscription);
        let operation = ValidatedOperation::new("{ ping }", None, &schema).unwrap();

        let result = operation.execute_sync(&graphql::vars! {}, &MyContext);

        assert_eq!(result, Ok((graphql::value!({"ping": true}), vec![])));
    }

    #[tokio::test]
    async fn asynchronously() {
        let schema = Schema::new(Query, Mutation, Subscription);
        let operation = ValidatedOperation::new("mutation { pong }", None, &schema).unwrap();

        let result = operation.execute(&graphql::vars! {}, &MyContext).await;

        assert_eq!(result, Ok((graphql::value!({"pong": true}), vec![])));
    }

    #[test]
    fn named_operation() {
        let schema = Schema::new(Query, Mutation, Subscription);
        let query = "query A { ping } query B { echo(value: 1) }";
        let operation = ValidatedOperation::new(query, Some("B"), &schema).unwrap();

        let result = operation.execute_sync(&graphql::vars! {}, &MyContext);

        assert_eq!(result, Ok((graphql::value!({"echo": 1}), vec![])));
    }

    #[test]
    fn fragments() {
        let schema = Schema::new(Query, Mutation, Subscription);
        let query =
            "query A { ...Echo } query B { ping } fragment Echo on Query { echo(value: 1) }";
        let operation = ValidatedOperation::new(query, Some("A"), &schema).unwrap();

        let result = operation.execute_sync(&graphql::vars! {}, &MyContext);

        assert_eq!(result, Ok((graphql::value!({"echo": 1}), vec![])));
    }

    #[test]
    fn default_values() {
        let schema = Schema::new(Query, Mutation, Subscription);
        let operation = ValidatedOperation::new(ECHO_WITH_DEFAULT, None, &schema).unwrap();

        let result = operation.execute_sync(&graphql::vars! {}, &MyContext);

        assert_eq!(result, Ok((graphql::value!({"echo": 3}), vec![])));
    }

    #[test]
    fn provided_variables_over_default_values() {
        let schema = Schema::new(Query, Mutation, Subscription);
        let operation = ValidatedOperation::new(ECHO_WITH_DEFAULT, None, &schema).unwrap();

        let result = operation.execute_sync(&graphql::vars! {"value": 5}, &MyContext);

        assert_eq!(result, Ok((graphql::value!({"echo": 5}), vec![])));
    }

    #[test]
    fn more_than_once() {
        let schema = Schema::new(Query, Mutation, Subscription);
        let operation = ValidatedOperation::new(ECHO, None, &schema).unwrap();

        for value in [1, 2] {
            let result = operation.execute_sync(&graphql::vars! {"value": (value)}, &MyContext);

            assert_eq!(result, Ok((graphql::value!({"echo": value}), vec![])));
        }
    }

    #[test]
    fn rejects_invalid_variables() {
        let schema = Schema::new(Query, Mutation, Subscription);
        let operation = ValidatedOperation::new(ECHO, None, &schema).unwrap();

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
        let schema = Schema::new(Query, Mutation, Subscription);
        let operation = ValidatedOperation::new("subscription { tick }", None, &schema).unwrap();

        let result = operation.execute_sync(&graphql::vars! {}, &MyContext);

        assert_eq!(result, Err(GraphQLError::IsSubscription));
    }
}

mod resolve_into_stream {
    use futures::StreamExt as _;

    use crate::{GraphQLError, ValidatedOperation, Value, graphql};

    use super::{Mutation, MyContext, Query, Schema, Subscription};

    #[tokio::test]
    async fn subscription() {
        let schema = Schema::new(Query, Mutation, Subscription);
        let operation = ValidatedOperation::new("subscription { tick }", None, &schema).unwrap();

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
        let schema = Schema::new(Query, Mutation, Subscription);
        let operation = ValidatedOperation::new("{ ping }", None, &schema).unwrap();

        let result = operation
            .resolve_into_stream(&graphql::vars! {}, &MyContext)
            .await;

        assert!(
            matches!(result, Err(GraphQLError::NotSubscription)),
            "expected `NotSubscription`",
        );
    }
}
