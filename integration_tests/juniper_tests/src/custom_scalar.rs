use std::{convert::TryInto as _, fmt, pin::Pin};

use futures::{stream, Stream};
use juniper::{
    execute, graphql_input_value, graphql_object, graphql_scalar, graphql_subscription,
    graphql_vars,
    parser::{ParseError, ScalarToken, Token},
    serde::{de, Deserialize, Deserializer, Serialize},
    EmptyMutation, FieldResult, InputValue, Object, ParseScalarResult, RootNode, ScalarValue,
    Value, Variables,
};

#[derive(Clone, Debug, PartialEq, ScalarValue, Serialize)]
#[serde(untagged)]
pub(crate) enum MyScalarValue {
    #[value(as_float, as_int)]
    Int(i32),
    Long(i64),
    #[value(as_float)]
    Float(f64),
    #[value(as_str, as_string, into_string)]
    String(String),
    #[value(as_bool)]
    Boolean(bool),
}

impl<'de> Deserialize<'de> for MyScalarValue {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = MyScalarValue;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a valid input value")
            }

            fn visit_bool<E: de::Error>(self, b: bool) -> Result<Self::Value, E> {
                Ok(MyScalarValue::Boolean(b))
            }

            fn visit_i32<E: de::Error>(self, n: i32) -> Result<Self::Value, E> {
                Ok(MyScalarValue::Int(n))
            }

            fn visit_i64<E: de::Error>(self, b: i64) -> Result<Self::Value, E> {
                if b <= i64::from(i32::MAX) {
                    self.visit_i32(b.try_into().unwrap())
                } else {
                    Ok(MyScalarValue::Long(b))
                }
            }

            fn visit_u32<E: de::Error>(self, n: u32) -> Result<Self::Value, E> {
                if n <= i32::MAX as u32 {
                    self.visit_i32(n.try_into().unwrap())
                } else {
                    self.visit_u64(n.into())
                }
            }

            fn visit_u64<E: de::Error>(self, n: u64) -> Result<Self::Value, E> {
                if n <= i64::MAX as u64 {
                    self.visit_i64(n.try_into().unwrap())
                } else {
                    // Browser's `JSON.stringify()` serializes all numbers
                    // having no fractional part as integers (no decimal point),
                    // so we must parse large integers as floating point,
                    // otherwise we would error on transferring large floating
                    // point numbers.
                    // TODO: Use `FloatToInt` conversion once stabilized:
                    //       https://github.com/rust-lang/rust/issues/67057
                    Ok(MyScalarValue::Float(n as f64))
                }
            }

            fn visit_f64<E: de::Error>(self, f: f64) -> Result<Self::Value, E> {
                Ok(MyScalarValue::Float(f))
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                self.visit_string(s.into())
            }

            fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
                Ok(MyScalarValue::String(s))
            }
        }

        de.deserialize_any(Visitor)
    }
}

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
            .ok_or_else(|| format!("Expected `MyScalarValue::Long`, found: {}", v))
    }

    pub(super) fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<'_, MyScalarValue> {
        if let ScalarToken::Int(v) = value {
            v.parse()
                .map_err(|_| ParseError::UnexpectedToken(Token::Scalar(value)))
                .map(|s: i64| s.into())
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
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
    F: Fn(&Object<MyScalarValue>) -> (),
{
    let schema =
        RootNode::new_with_scalar_value(TestType, EmptyMutation::<()>::new(), TestSubscriptionType);

    let (result, errs) = execute(query, None, &schema, &vars, &())
        .await
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let obj = result.as_object_value().expect("Result is not an object");

    f(obj);
}

async fn run_query<F>(query: &str, f: F)
where
    F: Fn(&Object<MyScalarValue>) -> (),
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
