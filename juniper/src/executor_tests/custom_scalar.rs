use ast::InputValue;
use executor::{ExecutionResult, Executor, Registry, Variables};
use parser::{ParseError, ScalarToken, Token};
use schema::model::RootNode;
use schema::meta::MetaType;
use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use std::fmt::{self, Display};
use types::base::{Arguments, GraphQLType};
use types::scalars::EmptyMutation;
use value::{Object, ScalarRefValue, ScalarValue, Value};

#[derive(Debug, Clone, PartialEq)]
enum MyScalarValue {
    Int(i32),
    Long(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

impl ScalarValue for MyScalarValue {}

impl Display for MyScalarValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MyScalarValue::Int(i) => write!(f, "{}", i),
            MyScalarValue::Long(i) => write!(f, "{}", i),
            MyScalarValue::Float(n) => write!(f, "{}", n),
            MyScalarValue::String(ref s) => write!(f, "\"{}\"", s),
            MyScalarValue::Boolean(b) => write!(f, "{}", b),
        }
    }
}

impl<'a> From<&'a str> for MyScalarValue {
    fn from(s: &'a str) -> Self {
        MyScalarValue::String(s.into())
    }
}

impl From<String> for MyScalarValue {
    fn from(s: String) -> Self {
        (&s as &str).into()
    }
}

impl From<bool> for MyScalarValue {
    fn from(b: bool) -> Self {
        MyScalarValue::Boolean(b)
    }
}

impl From<i32> for MyScalarValue {
    fn from(i: i32) -> Self {
        MyScalarValue::Int(i)
    }
}

impl From<i64> for MyScalarValue {
    fn from(i: i64) -> Self {
        MyScalarValue::Long(i)
    }
}

impl From<f64> for MyScalarValue {
    fn from(f: f64) -> Self {
        MyScalarValue::Float(f)
    }
}

impl From<MyScalarValue> for Option<bool> {
    fn from(s: MyScalarValue) -> Self {
        match s {
            MyScalarValue::Boolean(b) => Some(b),
            _ => None,
        }
    }
}

impl From<MyScalarValue> for Option<i32> {
    fn from(s: MyScalarValue) -> Self {
        match s {
            MyScalarValue::Int(i) => Some(i),
            _ => None,
        }
    }
}

impl From<MyScalarValue> for Option<f64> {
    fn from(s: MyScalarValue) -> Self {
        match s {
            MyScalarValue::Float(s) => Some(s),
            MyScalarValue::Int(i) => Some(i as f64),
            _ => None,
        }
    }
}

impl From<MyScalarValue> for Option<String> {
    fn from(s: MyScalarValue) -> Self {
        match s {
            MyScalarValue::String(s) => Some(s),
            _ => None,
        }
    }
}

impl<'a> From<&'a MyScalarValue> for Option<bool> {
    fn from(s: &'a MyScalarValue) -> Self {
        match *s {
            MyScalarValue::Boolean(b) => Some(b),
            _ => None,
        }
    }
}

impl<'a> From<&'a MyScalarValue> for Option<f64> {
    fn from(s: &'a MyScalarValue) -> Self {
        match *s {
            MyScalarValue::Float(b) => Some(b),
            MyScalarValue::Int(i) => Some(i as f64),
            _ => None,
        }
    }
}

impl<'a> From<&'a MyScalarValue> for Option<i32> {
    fn from(s: &'a MyScalarValue) -> Self {
        match *s {
            MyScalarValue::Int(b) => Some(b),
            _ => None,
        }
    }
}

impl<'a> From<&'a MyScalarValue> for Option<i64> {
    fn from(s: &'a MyScalarValue) -> Self {
        match *s {
            MyScalarValue::Long(l) => Some(l),
            _ => None,
        }
    }
}

impl<'a> From<&'a MyScalarValue> for Option<&'a str> {
    fn from(s: &'a MyScalarValue) -> Self {
        match *s {
            MyScalarValue::String(ref s) => Some(s),
            _ => None,
        }
    }
}

impl Serialize for MyScalarValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            MyScalarValue::Int(v) => serializer.serialize_i32(v),
            MyScalarValue::Long(v) => serializer.serialize_i64(v),
            MyScalarValue::Float(v) => serializer.serialize_f64(v),
            MyScalarValue::String(ref v) => serializer.serialize_str(v),
            MyScalarValue::Boolean(v) => serializer.serialize_bool(v),
        }
    }
}

impl<'de> Deserialize<'de> for MyScalarValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MyScalarValueVisitor;

        impl<'de> de::Visitor<'de> for MyScalarValueVisitor {
            type Value = MyScalarValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid input value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<MyScalarValue, E> {
                Ok(MyScalarValue::Boolean(value))
            }

            fn visit_i32<E>(self, value: i32) -> Result<MyScalarValue, E>
            where
                E: de::Error,
            {
                Ok(MyScalarValue::Int(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<MyScalarValue, E>
            where
                E: de::Error,
            {
                Ok(MyScalarValue::Long(value))
            }

            fn visit_u32<E>(self, value: u32) -> Result<MyScalarValue, E>
            where
                E: de::Error,
            {
                if value <= i32::max_value() as u32 {
                    self.visit_i32(value as i32)
                } else {
                    self.visit_u64(value as u64)
                }
            }

            fn visit_u64<E>(self, value: u64) -> Result<MyScalarValue, E>
            where
                E: de::Error,
            {
                if value <= i64::max_value() as u64 {
                    self.visit_i64(value as i64)
                } else {
                    // Browser's JSON.stringify serialize all numbers having no
                    // fractional part as integers (no decimal point), so we
                    // must parse large integers as floating point otherwise
                    // we would error on transferring large floating point
                    // numbers.
                    Ok(MyScalarValue::Float(value as f64))
                }
            }

            fn visit_f64<E>(self, value: f64) -> Result<MyScalarValue, E> {
                Ok(MyScalarValue::Float(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<MyScalarValue, E>
            where
                E: de::Error,
            {
                self.visit_string(value.into())
            }

            fn visit_string<E>(self, value: String) -> Result<MyScalarValue, E> {
                Ok(MyScalarValue::String(value))
            }
        }

        deserializer.deserialize_any(MyScalarValueVisitor)
    }
}

graphql_scalar!(i64 as "Long" where Scalar = MyScalarValue {
    resolve(&self) -> Value {
        Value::scalar(*self)
    }

    from_input_value(v: &InputValue) -> Option<i64> {
        match *v {
            InputValue::Scalar(ref i) => <_ as Into<Option<i64>>>::into(i),
            _ => None,
        }
    }

    from_str<'a>(value: ScalarToken<'a>) -> Result<MyScalarValue, ParseError<'a>> {
        if let ScalarToken::Int(v) = value {
                v.parse()
                    .map_err(|_| ParseError::UnexpectedToken(Token::Scalar(value)))
                    .map(|s: i64| s.into())
        } else {
                Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
});

struct TestType;

impl GraphQLType<MyScalarValue> for TestType {
    type Context = ();
    type TypeInfo = ();

    fn name((): &Self::TypeInfo) -> Option<&str> {
        Some("TestType")
    }

    fn meta<'r>(
        info: &Self::TypeInfo,
        registry: &mut Registry<'r, MyScalarValue>,
    ) -> MetaType<'r, MyScalarValue>
    where
        MyScalarValue: 'r,
        for<'b> &'b MyScalarValue: ScalarRefValue<'b>,
    {
        let long_field = registry.field::<i64>("longField", info);

        let long_arg = registry.arg::<i64>("longArg", info);

        let long_field_with_arg = registry
            .field::<i64>("longWithArg", info)
            .argument(long_arg);

        registry
            .build_object_type::<Self>(info, &[long_field, long_field_with_arg])
            .into_meta()
    }

    fn resolve_field(
        &self,
        _info: &Self::TypeInfo,
        field_name: &str,
        args: &Arguments<MyScalarValue>,
        _executor: &Executor<MyScalarValue, Self::Context>,
    ) -> ExecutionResult<MyScalarValue> {
        match field_name {
            "longField" => Ok(Value::Scalar(MyScalarValue::Long(
                (::std::i32::MAX as i64) + 1,
            ))),
            "longWithArg" => Ok(Value::Scalar(MyScalarValue::Long(
                args.get::<i64>("longArg").unwrap(),
            ))),
            _ => unreachable!(),
        }
    }
}

fn run_variable_query<F>(query: &str, vars: Variables<MyScalarValue>, f: F)
where
    F: Fn(&Object<MyScalarValue>) -> (),
{
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let (result, errs) = ::execute(query, None, &schema, &vars, &()).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let obj = result.as_object_value().expect("Result is not an object");

    f(obj);
}

fn run_query<F>(query: &str, f: F)
where
    F: Fn(&Object<MyScalarValue>) -> (),
{
    run_variable_query(query, Variables::new(), f);
}

#[test]
fn querying_long() {
    run_query("{ longField }", |result| {
        assert_eq!(
            result.get_field_value("longField"),
            Some(&Value::scalar((::std::i32::MAX as i64) + 1))
        );
    });
}

#[test]
fn querying_long_arg() {
    run_query(
        &format!(
            "{{ longWithArg(longArg: {}) }}",
            (::std::i32::MAX as i64) + 3
        ),
        |result| {
            assert_eq!(
                result.get_field_value("longWithArg"),
                Some(&Value::scalar((::std::i32::MAX as i64) + 3))
            );
        },
    );
}

#[test]
fn querying_long_variable() {
    run_variable_query(
        "query q($test: Long!){ longWithArg(longArg: $test) }",
        vec![(
            "test".to_owned(),
            InputValue::Scalar(MyScalarValue::Long((::std::i32::MAX as i64) + 42)),
        )].into_iter()
        .collect(),
        |result| {
            assert_eq!(
                result.get_field_value("longWithArg"),
                Some(&Value::scalar((::std::i32::MAX as i64) + 42))
            );
        },
    );
}
