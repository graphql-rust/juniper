use parser::{ParseError, ScalarToken};
use serde::de::{self, Deserialize, Deserializer};
use serde::ser::Serialize;
use std::fmt::{self, Debug, Display};

pub trait ParseScalarValue<S> {
    fn from_str<'a>(value: ScalarToken<'a>) -> Result<S, ParseError<'a>>;
}

pub trait ScalarValue:
    Debug
    + Display
    + PartialEq
    + Clone
    + Serialize
    + From<String>
    + From<bool>
    + From<i32>
    + From<f64>
    + Into<Option<bool>>
    + Into<Option<i32>>
    + Into<Option<f64>>
    + Into<Option<String>>
{
    fn is_type<'a, T>(&'a self) -> bool
    where
        &'a Self: Into<Option<T>>,
    {
        self.into().is_some()
    }
}

pub trait ScalarRefValue<'a>:
    Debug
    + Into<Option<&'a bool>>
    + Into<Option<&'a i32>>
    + Into<Option<&'a String>>
    + Into<Option<&'a f64>>
{
}

impl<'a, T> ScalarRefValue<'a> for &'a T
where
    T: ScalarValue,
    &'a T: Into<Option<&'a bool>>
        + Into<Option<&'a i32>>
        + Into<Option<&'a String>>
        + Into<Option<&'a f64>>,
{}

#[derive(Debug, PartialEq, Clone, ScalarValue)]
pub enum DefaultScalarValue {
    Int(i32),
    Float(f64),
    String(String),
    Boolean(bool),
}

impl<'de> Deserialize<'de> for DefaultScalarValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DefaultScalarValueVisitor;

        impl<'de> de::Visitor<'de> for DefaultScalarValueVisitor {
            type Value = DefaultScalarValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid input value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<DefaultScalarValue, E> {
                Ok(DefaultScalarValue::Boolean(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<DefaultScalarValue, E>
            where
                E: de::Error,
            {
                if value >= i64::from(i32::min_value()) && value <= i64::from(i32::max_value()) {
                    Ok(DefaultScalarValue::Int(value as i32))
                } else {
                    // Browser's JSON.stringify serialize all numbers having no
                    // fractional part as integers (no decimal point), so we
                    // must parse large integers as floating point otherwise
                    // we would error on transferring large floating point
                    // numbers.
                    Ok(DefaultScalarValue::Float(value as f64))
                }
            }

            fn visit_u64<E>(self, value: u64) -> Result<DefaultScalarValue, E>
            where
                E: de::Error,
            {
                if value <= i32::max_value() as u64 {
                    self.visit_i64(value as i64)
                } else {
                    // Browser's JSON.stringify serialize all numbers having no
                    // fractional part as integers (no decimal point), so we
                    // must parse large integers as floating point otherwise
                    // we would error on transferring large floating point
                    // numbers.
                    Ok(DefaultScalarValue::Float(value as f64))
                }
            }

            fn visit_f64<E>(self, value: f64) -> Result<DefaultScalarValue, E> {
                Ok(DefaultScalarValue::Float(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<DefaultScalarValue, E>
            where
                E: de::Error,
            {
                self.visit_string(value.into())
            }

            fn visit_string<E>(self, value: String) -> Result<DefaultScalarValue, E> {
                Ok(DefaultScalarValue::String(value))
            }
        }

        deserializer.deserialize_any(DefaultScalarValueVisitor)
    }
}
