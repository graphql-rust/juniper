use parser::{ParseError, ScalarToken};
use serde::de::{self, Deserialize, Deserializer};
use serde::ser::Serialize;
use std::fmt::{self, Debug, Display};

/// The result of converting a string into a scalar value
pub type ParseScalarResult<'a, S = DefaultScalarValue> = Result<S, ParseError<'a>>;

/// A trait used to convert a `ScalarToken` into a certain scalar value type
pub trait ParseScalarValue<S = DefaultScalarValue> {
    /// See the trait documentation
    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S>;
}

/// A trait marking a type that could be used as internal representation of
/// scalar values in juniper
///
/// The main objective of this abstraction is to allow other libraries to
/// replace the default representation with something that better fits thei
/// needs.
/// There is a custom derive (`#[derive(ScalarValue)]`) available that implements
/// most of the required traits automatically for a enum representing a scalar value.
/// The only trait that needs to be implemented manually in this case is `serde::Deserialize`.
///
/// # Implementing a new scalar value representation
/// The preferred way to define a new scalar value representation is
/// defining a enum containing a variant for each type that needs to be represented
/// at the lowest level.
/// The following example introduces an new variant that is able to store 64 bit integers.
///
/// ```
/// # #[macro_use]
/// # extern crate juniper;
/// # extern crate serde;
/// # use serde::{de, Deserialize, Deserializer};
/// # use std::fmt;
/// #
/// #[derive(Debug, Clone, PartialEq, ScalarValue)]
/// enum MyScalarValue {
///     Int(i32),
///     Long(i64),
///     Float(f64),
///     String(String),
///     Boolean(bool),
/// }
///
/// impl<'de> Deserialize<'de> for MyScalarValue {
///     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
///     where
///         D: Deserializer<'de>,
///     {
///         struct MyScalarValueVisitor;
///
///         impl<'de> de::Visitor<'de> for MyScalarValueVisitor {
///             type Value = MyScalarValue;
///
///             fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
///                 formatter.write_str("a valid input value")
///             }
///
///             fn visit_bool<E>(self, value: bool) -> Result<MyScalarValue, E> {
///                 Ok(MyScalarValue::Boolean(value))
///             }
///
///             fn visit_i32<E>(self, value: i32) -> Result<MyScalarValue, E>
///             where
///                 E: de::Error,
///             {
///                 Ok(MyScalarValue::Int(value))
///             }
///
///             fn visit_i64<E>(self, value: i64) -> Result<MyScalarValue, E>
///             where
///                 E: de::Error,
///             {
///                 Ok(MyScalarValue::Long(value))
///             }
///
///             fn visit_u32<E>(self, value: u32) -> Result<MyScalarValue, E>
///             where
///                 E: de::Error,
///             {
///                 if value <= i32::max_value() as u32 {
///                     self.visit_i32(value as i32)
///                 } else {
///                     self.visit_u64(value as u64)
///                 }
///             }
///
///             fn visit_u64<E>(self, value: u64) -> Result<MyScalarValue, E>
///             where
///                 E: de::Error,
///             {
///                 if value <= i64::max_value() as u64 {
///                     self.visit_i64(value as i64)
///                 } else {
///                     // Browser's JSON.stringify serialize all numbers having no
///                     // fractional part as integers (no decimal point), so we
///                     // must parse large integers as floating point otherwise
///                     // we would error on transferring large floating point
///                     // numbers.
///                     Ok(MyScalarValue::Float(value as f64))
///                 }
///             }
///
///             fn visit_f64<E>(self, value: f64) -> Result<MyScalarValue, E> {
///                 Ok(MyScalarValue::Float(value))
///             }
///
///             fn visit_str<E>(self, value: &str) -> Result<MyScalarValue, E>
///             where
///                 E: de::Error,
///             {
///                 self.visit_string(value.into())
///             }
///
///             fn visit_string<E>(self, value: String) -> Result<MyScalarValue, E> {
///                 Ok(MyScalarValue::String(value))
///             }
///         }
///
///         deserializer.deserialize_any(MyScalarValueVisitor)
///     }
/// }
/// # fn main() {}
/// ```
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
}

/// A marker trait extending the [`ScalarValue`](../trait.ScalarValue.html) trait
///
/// This trait should not be relied on directly by most apps.  However, you may
/// need a where clause in the form of `for<'b> &'b S: ScalarRefValue<'b>` to
/// abstract over different scalar value types.
///
/// This is automatically implemented for a type as soon as the type implements
/// `ScalarValue` and the additional conversations.
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

/// The default scalar value representation in juniper
///
/// This types closely follows the graphql specification.
#[derive(Debug, PartialEq, Clone, ScalarValue)]
#[allow(missing_docs)]
pub enum DefaultScalarValue {
    Int(i32),
    Float(f64),
    String(String),
    Boolean(bool),
}

impl<'a> From<&'a str> for DefaultScalarValue {
    fn from(s: &'a str) -> Self {
        DefaultScalarValue::String(s.into())
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
