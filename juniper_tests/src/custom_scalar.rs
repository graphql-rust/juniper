extern crate serde_json;

#[cfg(test)]
use juniper::parser::Spanning;
use juniper::parser::{ParseError, ScalarToken, Token};
use juniper::serde::de;
#[cfg(test)]
use juniper::{execute, EmptyMutation, Object, RootNode, Variables};
use juniper::{InputValue, ParseScalarResult, ScalarValue, Value};
use std::fmt;

#[derive(Debug, Clone, PartialEq, ScalarValue)]
enum MyScalarValue {
    Int(i32),
    Long(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

impl ScalarValue for MyScalarValue {
    type Visitor = MyScalarValueVisitor;

    fn as_int(&self) -> Option<i32> {
        match *self {
            MyScalarValue::Int(ref i) => Some(*i),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<String> {
        match *self {
            MyScalarValue::String(ref s) => Some(s.clone()),
            _ => None,
        }
    }

    fn as_float(&self) -> Option<f64> {
        match *self {
            MyScalarValue::Int(ref i) => Some(*i as f64),
            MyScalarValue::Float(ref f) => Some(*f),
            _ => None,
        }
    }

    fn as_boolean(&self) -> Option<bool> {
        match *self {
            MyScalarValue::Boolean(ref b) => Some(*b),
            _ => None,
        }
    }
}

#[derive(Default, Debug)]
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
        if value <= i32::max_value() as i64 {
            self.visit_i32(value as i32)
        } else {
            Ok(MyScalarValue::Long(value))
        }
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

graphql_scalar!(i64 as "Long" where Scalar = MyScalarValue {
    resolve(&self) -> Value {
        Value::scalar(*self)
    }

    from_input_value(v: &InputValue) -> Option<i64> {
        match *v {
            InputValue::Scalar(MyScalarValue::Long(i)) => Some(i),
            _ => None,
        }
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, MyScalarValue> {
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

graphql_object!(TestType: () where Scalar = MyScalarValue |&self| {
    field long_field()  -> i64 {
        (::std::i32::MAX as i64) + 1
    }

    field long_with_arg(long_arg: i64) -> i64 {
        long_arg
    }
});

#[cfg(test)]
fn run_variable_query<F>(query: &str, vars: Variables<MyScalarValue>, f: F)
where
    F: Fn(&Object<MyScalarValue>) -> (),
{
    let schema = RootNode::new(TestType, EmptyMutation::<()>::new());

    let (result, errs) = execute(query, None, &schema, &vars, &()).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let obj = result.as_object_value().expect("Result is not an object");

    f(obj);
}

#[cfg(test)]
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

#[test]
fn deserialize_variable() {
    let json = format!("{{\"field\": {}}}", (::std::i32::MAX as i64) + 42);

    let input_value: InputValue<MyScalarValue> = self::serde_json::from_str(&json).unwrap();
    assert_eq!(
        input_value,
        InputValue::Object(vec![(
            Spanning::unlocated("field".into()),
            Spanning::unlocated(InputValue::Scalar(MyScalarValue::Long(
                (::std::i32::MAX as i64) + 42
            )))
        )])
    );
}
