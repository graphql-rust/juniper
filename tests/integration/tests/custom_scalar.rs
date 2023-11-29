pub mod common;

use std::pin::Pin;

use futures::{stream, Stream};
use juniper::{
    execute, graphql_input_value, graphql_object, graphql_scalar, graphql_subscription,
    graphql_vars,
    parser::{ParseError, ScalarToken, Token},
    EmptyMutation, FieldResult, InputValue, Object, ParseScalarResult, RootNode, Value, Variables,
};

use self::common::MyScalarValue;

#[graphql_scalar(with = long, scalar = MyScalarValue)]
type Long = i64;

mod long {
    use super::*;

    pub(super) fn to_output(v: &Long) -> Value<MyScalarValue> {
        Value::scalar(*v)
    }

    pub(super) fn from_input(v: &InputValue<MyScalarValue>) -> Result<Long, String> {
        v.as_scalar_value::<i64>()
            .copied()
            .ok_or_else(|| format!("Expected `MyScalarValue::Long`, found: {v}"))
    }

    pub(super) fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<MyScalarValue> {
        if let ScalarToken::Int(v) = value {
            v.parse()
                .map_err(|_| ParseError::unexpected_token(Token::Scalar(value)))
                .map(|s: i64| s.into())
        } else {
            Err(ParseError::unexpected_token(Token::Scalar(value)))
        }
    }
}

struct TestType;

#[graphql_object(scalar = MyScalarValue)]
impl TestType {
    fn long_field() -> i64 {
        i64::from(i32::MAX) + 1
    }

    fn long_with_arg(long_arg: i64) -> i64 {
        long_arg
    }
}

struct TestSubscriptionType;

#[graphql_subscription(scalar = MyScalarValue)]
impl TestSubscriptionType {
    async fn foo() -> Pin<Box<dyn Stream<Item = FieldResult<i32, MyScalarValue>> + Send>> {
        Box::pin(stream::empty())
    }
}

async fn run_variable_query<F>(query: &str, vars: Variables<MyScalarValue>, f: F)
where
    F: Fn(&Object<MyScalarValue>),
{
    let schema =
        RootNode::new_with_scalar_value(TestType, EmptyMutation::<()>::new(), TestSubscriptionType);

    let (result, errs) = execute(query, None, &schema, &vars, &())
        .await
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {result:?}");

    let obj = result.as_object_value().expect("Result is not an object");

    f(obj);
}

async fn run_query<F>(query: &str, f: F)
where
    F: Fn(&Object<MyScalarValue>),
{
    run_variable_query(query, graphql_vars! {}, f).await;
}

#[tokio::test]
async fn querying_long() {
    run_query("{ longField }", |result| {
        assert_eq!(
            result.get_field_value("longField"),
            Some(&Value::scalar(i64::from(i32::MAX) + 1))
        );
    })
    .await;
}

#[tokio::test]
async fn querying_long_arg() {
    run_query(
        &format!("{{ longWithArg(longArg: {}) }}", i64::from(i32::MAX) + 3),
        |result| {
            assert_eq!(
                result.get_field_value("longWithArg"),
                Some(&Value::scalar(i64::from(i32::MAX) + 3))
            );
        },
    )
    .await;
}

#[tokio::test]
async fn querying_long_variable() {
    let num = i64::from(i32::MAX) + 42;
    run_variable_query(
        "query q($test: Long!){ longWithArg(longArg: $test) }",
        graphql_vars! {"test": InputValue::<_>::scalar(num)},
        |result| {
            assert_eq!(
                result.get_field_value("longWithArg"),
                Some(&Value::scalar(num)),
            );
        },
    )
    .await;
}

#[test]
fn deserialize_variable() {
    let json = format!("{{\"field\": {}}}", i64::from(i32::MAX) + 42);

    assert_eq!(
        serde_json::from_str::<InputValue<MyScalarValue>>(&json).unwrap(),
        graphql_input_value!({
            "field": InputValue::<_>::scalar(i64::from(i32::MAX) + 42),
        }),
    );
}
