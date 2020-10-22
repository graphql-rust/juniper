use std::fmt;

use serde::{de, ser::Serialize};

use crate::{
    parser::{ParseError, ScalarToken},
    GraphQLScalarValue,
};

/// The result of converting a string into a scalar value
pub type ParseScalarResult<'a, S = DefaultScalarValue> = Result<S, ParseError<'a>>;

/// A trait used to convert a `ScalarToken` into a certain scalar value type
pub trait ParseScalarValue<S = DefaultScalarValue> {
    /// See the trait documentation
    fn from_str(value: ScalarToken<'_>) -> ParseScalarResult<'_, S>;
}

/// A trait marking a type that could be used as internal representation of
/// scalar values in juniper
///
/// The main objective of this abstraction is to allow other libraries to
/// replace the default representation with something that better fits their
/// needs.
/// There is a custom derive (`#[derive(juniper::GraphQLScalarValue)]`) available that implements
/// most of the required traits automatically for a enum representing a scalar value.
/// This derives needs a additional annotation of the form
/// `#[juniper(visitor = "VisitorType")]` to specify a type that implements
/// `serde::de::Visitor` and that is used to deserialize the value.
///
/// # Implementing a new scalar value representation
/// The preferred way to define a new scalar value representation is
/// defining a enum containing a variant for each type that needs to be represented
/// at the lowest level.
/// The following example introduces an new variant that is able to store 64 bit integers.
///
/// ```
/// # use std::fmt;
/// # use serde::{de, Deserialize, Deserializer};
/// # use juniper::ScalarValue;
/// #
/// #[derive(Debug, Clone, PartialEq, juniper::GraphQLScalarValue)]
/// enum MyScalarValue {
///     Int(i32),
///     Long(i64),
///     Float(f64),
///     String(String),
///     Boolean(bool),
/// }
///
/// impl ScalarValue for MyScalarValue {
///     type Visitor = MyScalarValueVisitor;
///
///      fn as_int(&self) -> Option<i32> {
///        match *self {
///            Self::Int(ref i) => Some(*i),
///            _ => None,
///        }
///    }
///
///    fn as_string(&self) -> Option<String> {
///        match *self {
///            Self::String(ref s) => Some(s.clone()),
///            _ => None,
///        }
///    }
///
///    fn into_string(self) -> Option<String> {
///        match self {
///            Self::String(s) => Some(s),
///            _ => None,
///        }
///    }
///
///    fn as_str(&self) -> Option<&str> {
///        match *self {
///            Self::String(ref s) => Some(s.as_str()),
///            _ => None,
///        }
///    }
///
///    fn as_float(&self) -> Option<f64> {
///        match *self {
///            Self::Int(ref i) => Some(*i as f64),
///            Self::Float(ref f) => Some(*f),
///            _ => None,
///        }
///    }
///
///    fn as_boolean(&self) -> Option<bool> {
///        match *self {
///            Self::Boolean(ref b) => Some(*b),
///            _ => None,
///        }
///    }
/// }
///
/// #[derive(Default)]
/// struct MyScalarValueVisitor;
///
/// impl<'de> de::Visitor<'de> for MyScalarValueVisitor {
///     type Value = MyScalarValue;
///
///     fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
///         formatter.write_str("a valid input value")
///     }
///
///     fn visit_bool<E>(self, value: bool) -> Result<MyScalarValue, E> {
///         Ok(MyScalarValue::Boolean(value))
///     }
///
///     fn visit_i32<E>(self, value: i32) -> Result<MyScalarValue, E>
///     where
///         E: de::Error,
///     {
///         Ok(MyScalarValue::Int(value))
///     }
///
///     fn visit_i64<E>(self, value: i64) -> Result<MyScalarValue, E>
///     where
///         E: de::Error,
///     {
///         if value <= i32::max_value() as i64 {
///             self.visit_i32(value as i32)
///         } else {
///             Ok(MyScalarValue::Long(value))
///         }
///     }
///
///     fn visit_u32<E>(self, value: u32) -> Result<MyScalarValue, E>
///     where
///         E: de::Error,
///     {
///         if value <= i32::max_value() as u32 {
///             self.visit_i32(value as i32)
///         } else {
///             self.visit_u64(value as u64)
///         }
///     }
///
///     fn visit_u64<E>(self, value: u64) -> Result<MyScalarValue, E>
///     where
///         E: de::Error,
///     {
///         if value <= i64::max_value() as u64 {
///             self.visit_i64(value as i64)
///         } else {
///             // Browser's JSON.stringify serialize all numbers having no
///             // fractional part as integers (no decimal point), so we
///             // must parse large integers as floating point otherwise
///             // we would error on transferring large floating point
///             // numbers.
///             Ok(MyScalarValue::Float(value as f64))
///         }
///     }
///
///     fn visit_f64<E>(self, value: f64) -> Result<MyScalarValue, E> {
///         Ok(MyScalarValue::Float(value))
///     }
///
///     fn visit_str<E>(self, value: &str) -> Result<MyScalarValue, E>
///     where
///         E: de::Error,
///     {
///         self.visit_string(value.into())
///     }
///
///     fn visit_string<E>(self, value: String) -> Result<MyScalarValue, E> {
///         Ok(MyScalarValue::String(value))
///     }
/// }
///
/// # fn main() {}
/// ```
pub trait ScalarValue:
    fmt::Debug
    + fmt::Display
    + PartialEq
    + Clone
    + Serialize
    + From<String>
    + From<bool>
    + From<i32>
    + From<f64>
    + 'static
{
    /// Serde visitor used to deserialize this scalar value
    type Visitor: for<'de> de::Visitor<'de, Value = Self> + Default;

    /// Checks if the current value contains the a value of the current type
    ///
    /// ```
    /// # use juniper::{ScalarValue, DefaultScalarValue};
    ///
    /// let value = DefaultScalarValue::Int(42);
    ///
    /// assert_eq!(value.is_type::<i32>(), true);
    /// assert_eq!(value.is_type::<f64>(), false);
    ///
    /// ```
    fn is_type<'a, T>(&'a self) -> bool
    where
        T: 'a,
        &'a Self: Into<Option<&'a T>>,
    {
        self.into().is_some()
    }

    /// Convert the given scalar value into an integer value
    ///
    /// This function is used for implementing `GraphQLValue` for `i32` for all
    /// scalar values. Implementations should convert all supported integer
    /// types with 32 bit or less to an integer if requested.
    fn as_int(&self) -> Option<i32>;

    /// Represents this [`ScalarValue`] a [`String`] value.
    ///
    /// This function is used for implementing `GraphQLValue` for `String` for all
    /// scalar values
    fn as_string(&self) -> Option<String>;

    /// Converts this [`ScalarValue`] into a [`String`] value.
    ///
    /// Same as [`ScalarValue::as_string`], but takes ownership, so allows to omit redundant
    /// cloning.
    fn into_string(self) -> Option<String>;

    /// Convert the given scalar value into a string value
    ///
    /// This function is used for implementing `GraphQLValue` for `String` for all
    /// scalar values
    fn as_str(&self) -> Option<&str>;

    /// Convert the given scalar value into a float value
    ///
    /// This function is used for implementing `GraphQLValue` for `f64` for all
    /// scalar values. Implementations should convert all supported integer
    /// types with 64 bit or less and all floating point values with 64 bit or
    /// less to a float if requested.
    fn as_float(&self) -> Option<f64>;

    /// Convert the given scalar value into a boolean value
    ///
    /// This function is used for implementing `GraphQLValue` for `bool` for all
    /// scalar values.
    fn as_boolean(&self) -> Option<bool>;

    /// Converts this [`ScalarValue`] into another one.
    fn into_another<S: ScalarValue>(self) -> S {
        if let Some(i) = self.as_int() {
            S::from(i)
        } else if let Some(f) = self.as_float() {
            S::from(f)
        } else if let Some(b) = self.as_boolean() {
            S::from(b)
        } else if let Some(s) = self.into_string() {
            S::from(s)
        } else {
            unreachable!("`ScalarValue` must represent at least one of the GraphQL spec types")
        }
    }
}

/// The default scalar value representation in juniper
///
/// This types closely follows the graphql specification.
#[derive(Debug, PartialEq, Clone, GraphQLScalarValue)]
#[allow(missing_docs)]
pub enum DefaultScalarValue {
    Int(i32),
    Float(f64),
    String(String),
    Boolean(bool),
}

impl ScalarValue for DefaultScalarValue {
    type Visitor = DefaultScalarValueVisitor;

    fn as_int(&self) -> Option<i32> {
        match *self {
            Self::Int(ref i) => Some(*i),
            _ => None,
        }
    }

    fn as_float(&self) -> Option<f64> {
        match *self {
            Self::Int(ref i) => Some(*i as f64),
            Self::Float(ref f) => Some(*f),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match *self {
            Self::String(ref s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<String> {
        match *self {
            Self::String(ref s) => Some(s.clone()),
            _ => None,
        }
    }

    fn into_string(self) -> Option<String> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    fn as_boolean(&self) -> Option<bool> {
        match *self {
            Self::Boolean(ref b) => Some(*b),
            _ => None,
        }
    }

    fn into_another<S: ScalarValue>(self) -> S {
        match self {
            Self::Int(i) => S::from(i),
            Self::Float(f) => S::from(f),
            Self::String(s) => S::from(s),
            Self::Boolean(b) => S::from(b),
        }
    }
}

impl<'a> From<&'a str> for DefaultScalarValue {
    fn from(s: &'a str) -> Self {
        Self::String(s.into())
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct DefaultScalarValueVisitor;

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
