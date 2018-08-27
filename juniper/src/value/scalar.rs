use parser::ParseError;
use schema::meta::ScalarMeta;
use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use std::fmt::{self, Debug, Display};

pub trait ParseScalarValue<S> {
    fn from_str(value: &str) -> Result<S, ParseError>;
}

pub trait ScalarValue:
    Debug
    + Display
    + PartialEq
    + Clone
    + Serialize
    + for<'a> From<&'a str>
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
    Debug + Into<Option<bool>> + Into<Option<i32>> + Into<Option<&'a str>> + Into<Option<f64>>
{
}

impl<'a, T> ScalarRefValue<'a> for &'a T
where
    T: ScalarValue,
    &'a T: Into<Option<bool>> + Into<Option<i32>> + Into<Option<&'a str>> + Into<Option<f64>>,
{}

#[derive(Debug, PartialEq, Clone)]
pub enum DefaultScalarValue {
    Int(i32),
    Float(f64),
    String(String),
    Boolean(bool),
}

impl ScalarValue for DefaultScalarValue {}

impl Display for DefaultScalarValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DefaultScalarValue::Int(i) => write!(f, "{}", i),
            DefaultScalarValue::Float(n) => write!(f, "{}", n),
            DefaultScalarValue::String(ref s) => write!(f, "\"{}\"", s),
            DefaultScalarValue::Boolean(b) => write!(f, "{}", b),
        }
    }
}

impl<'a> From<&'a str> for DefaultScalarValue {
    fn from(s: &'a str) -> Self {
        DefaultScalarValue::String(s.into())
    }
}

impl From<String> for DefaultScalarValue {
    fn from(s: String) -> Self {
        (&s as &str).into()
    }
}

impl From<bool> for DefaultScalarValue {
    fn from(b: bool) -> Self {
        DefaultScalarValue::Boolean(b)
    }
}

impl From<i32> for DefaultScalarValue {
    fn from(i: i32) -> Self {
        DefaultScalarValue::Int(i)
    }
}

impl From<f64> for DefaultScalarValue {
    fn from(f: f64) -> Self {
        DefaultScalarValue::Float(f)
    }
}

impl From<DefaultScalarValue> for Option<bool> {
    fn from(s: DefaultScalarValue) -> Self {
        match s {
            DefaultScalarValue::Boolean(b) => Some(b),
            _ => None,
        }
    }
}

impl From<DefaultScalarValue> for Option<i32> {
    fn from(s: DefaultScalarValue) -> Self {
        match s {
            DefaultScalarValue::Int(i) => Some(i),
            _ => None,
        }
    }
}

impl From<DefaultScalarValue> for Option<f64> {
    fn from(s: DefaultScalarValue) -> Self {
        match s {
            DefaultScalarValue::Float(s) => Some(s),
            DefaultScalarValue::Int(i) => Some(i as f64),
            _ => None,
        }
    }
}

impl From<DefaultScalarValue> for Option<String> {
    fn from(s: DefaultScalarValue) -> Self {
        match s {
            DefaultScalarValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }
}

impl<'a> From<&'a DefaultScalarValue> for Option<bool> {
    fn from(s: &'a DefaultScalarValue) -> Self {
        match *s {
            DefaultScalarValue::Boolean(b) => Some(b),
            _ => None,
        }
    }
}

impl<'a> From<&'a DefaultScalarValue> for Option<f64> {
    fn from(s: &'a DefaultScalarValue) -> Self {
        match *s {
            DefaultScalarValue::Float(b) => Some(b),
            DefaultScalarValue::Int(i) => Some(i as f64),
            _ => None,
        }
    }
}

impl<'a> From<&'a DefaultScalarValue> for Option<i32> {
    fn from(s: &'a DefaultScalarValue) -> Self {
        match *s {
            DefaultScalarValue::Int(b) => Some(b),
            _ => None,
        }
    }
}

impl<'a> From<&'a DefaultScalarValue> for Option<&'a str> {
    fn from(s: &'a DefaultScalarValue) -> Self {
        match *s {
            DefaultScalarValue::String(ref s) => Some(s),
            _ => None,
        }
    }
}

impl Serialize for DefaultScalarValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            DefaultScalarValue::Int(v) => serializer.serialize_i32(v),
            DefaultScalarValue::Float(v) => serializer.serialize_f64(v),
            DefaultScalarValue::String(ref v) => serializer.serialize_str(v),
            DefaultScalarValue::Boolean(v) => serializer.serialize_bool(v),
        }
    }
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
