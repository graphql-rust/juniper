use std::{borrow::Cow, fmt};

use serde::{de::DeserializeOwned, Serialize};

use crate::parser::{ParseError, ScalarToken};

pub use juniper_codegen::ScalarValue;

/// The result of converting a string into a scalar value
pub type ParseScalarResult<S = DefaultScalarValue> = Result<S, ParseError>;

/// A trait used to convert a `ScalarToken` into a certain scalar value type
pub trait ParseScalarValue<S = DefaultScalarValue> {
    /// See the trait documentation
    fn from_str(value: ScalarToken<'_>) -> ParseScalarResult<S>;
}

/// A trait marking a type that could be used as internal representation of
/// scalar values in juniper
///
/// The main objective of this abstraction is to allow other libraries to
/// replace the default representation with something that better fits their
/// needs.
/// There is a custom derive (`#[derive(`[`ScalarValue`]`)]`) available that
/// implements most of the required traits automatically for a enum representing
/// a scalar value. However, [`Serialize`] and [`Deserialize`] implementations
/// are expected to be provided.
///
/// # Implementing a new scalar value representation
/// The preferred way to define a new scalar value representation is
/// defining a enum containing a variant for each type that needs to be
/// represented at the lowest level.
/// The following example introduces an new variant that is able to store 64 bit
/// integers.
///
/// ```rust
/// # use std::fmt;
/// #
/// # use serde::{de, Deserialize, Deserializer, Serialize};
/// # use juniper::ScalarValue;
/// #
/// #[derive(Clone, Debug, PartialEq, ScalarValue, Serialize)]
/// #[serde(untagged)]
/// enum MyScalarValue {
///     #[value(as_float, as_int)]
///     Int(i32),
///     Long(i64),
///     #[value(as_float)]
///     Float(f64),
///     #[value(as_str, as_string, into_string)]
///     String(String),
///     #[value(as_bool)]
///     Boolean(bool),
/// }
///
/// impl<'de> Deserialize<'de> for MyScalarValue {
///     fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
///         struct Visitor;
///
///         impl<'de> de::Visitor<'de> for Visitor {
///             type Value = MyScalarValue;
///
///             fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
///                 f.write_str("a valid input value")
///             }
///
///             fn visit_bool<E: de::Error>(self, b: bool) -> Result<Self::Value, E> {
///                 Ok(MyScalarValue::Boolean(b))
///             }
///
///             fn visit_i32<E: de::Error>(self, n: i32) -> Result<Self::Value, E> {
///                 Ok(MyScalarValue::Int(n))
///             }
///
///             fn visit_i64<E: de::Error>(self, n: i64) -> Result<Self::Value, E> {
///                 if n <= i64::from(i32::MAX) {
///                     self.visit_i32(n.try_into().unwrap())
///                 } else {
///                     Ok(MyScalarValue::Long(n))
///                 }
///             }
///
///             fn visit_u32<E: de::Error>(self, n: u32) -> Result<Self::Value, E> {
///                 if n <= i32::MAX as u32 {
///                     self.visit_i32(n.try_into().unwrap())
///                 } else {
///                     self.visit_u64(n.into())
///                 }
///             }
///
///             fn visit_u64<E: de::Error>(self, n: u64) -> Result<Self::Value, E> {
///                 if n <= i64::MAX as u64 {
///                     self.visit_i64(n.try_into().unwrap())
///                 } else {
///                     // Browser's `JSON.stringify()` serialize all numbers
///                     // having no fractional part as integers (no decimal
///                     // point), so we must parse large integers as floating
///                     // point, otherwise we would error on transferring large
///                     // floating point numbers.
///                     Ok(MyScalarValue::Float(n as f64))
///                 }
///             }
///
///             fn visit_f64<E: de::Error>(self, f: f64) -> Result<Self::Value, E> {
///                 Ok(MyScalarValue::Float(f))
///             }
///
///             fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
///                 self.visit_string(s.into())
///             }
///
///             fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
///                 Ok(MyScalarValue::String(s))
///             }
///         }
///
///         de.deserialize_any(Visitor)
///     }
/// }
/// ```
///
/// [`Deserialize`]: trait@serde::Deserialize
/// [`Serialize`]: trait@serde::Serialize
pub trait ScalarValue:
    fmt::Debug
    + fmt::Display
    + PartialEq
    + Clone
    + DeserializeOwned
    + Serialize
    + From<String>
    + From<bool>
    + From<i32>
    + From<f64>
    + 'static
{
    /// Checks whether this [`ScalarValue`] contains the value of the given
    /// type.
    ///
    /// ```
    /// # use juniper::{ScalarValue, DefaultScalarValue};
    /// #
    /// let value = DefaultScalarValue::Int(42);
    ///
    /// assert_eq!(value.is_type::<i32>(), true);
    /// assert_eq!(value.is_type::<f64>(), false);
    /// ```
    #[must_use]
    fn is_type<'a, T>(&'a self) -> bool
    where
        T: 'a,
        Option<&'a T>: From<&'a Self>,
    {
        <Option<&'a T>>::from(self).is_some()
    }

    /// Represents this [`ScalarValue`] as an integer value.
    ///
    /// This function is used for implementing [`GraphQLValue`] for [`i32`] for
    /// all possible [`ScalarValue`]s. Implementations should convert all the
    /// supported integer types with 32 bit or less to an integer, if requested.
    ///
    /// [`GraphQLValue`]: crate::GraphQLValue
    #[must_use]
    fn as_int(&self) -> Option<i32>;

    /// Represents this [`ScalarValue`] as a [`String`] value.
    ///
    /// This function is used for implementing [`GraphQLValue`] for [`String`]
    /// for all possible [`ScalarValue`]s.
    ///
    /// [`GraphQLValue`]: crate::GraphQLValue
    #[must_use]
    fn as_string(&self) -> Option<String>;

    /// Converts this [`ScalarValue`] into a [`String`] value.
    ///
    /// Same as [`ScalarValue::as_string()`], but takes ownership, so allows to
    /// omit redundant cloning.
    #[must_use]
    fn into_string(self) -> Option<String>;

    /// Represents this [`ScalarValue`] as a [`str`] value.
    ///
    /// This function is used for implementing [`GraphQLValue`] for [`str`] for
    /// all possible [`ScalarValue`]s.
    ///
    /// [`GraphQLValue`]: crate::GraphQLValue
    #[must_use]
    fn as_str(&self) -> Option<&str>;

    /// Represents this [`ScalarValue`] as a float value.
    ///
    /// This function is used for implementing [`GraphQLValue`] for [`f64`] for
    /// all possible [`ScalarValue`]s. Implementations should convert all
    /// supported integer types with 64 bit or less and all floating point
    /// values with 64 bit or less to a float, if requested.
    ///
    /// [`GraphQLValue`]: crate::GraphQLValue
    #[must_use]
    fn as_float(&self) -> Option<f64>;

    /// Represents this [`ScalarValue`] as a boolean value
    ///
    /// This function is used for implementing [`GraphQLValue`] for [`bool`] for
    /// all possible [`ScalarValue`]s.
    ///
    /// [`GraphQLValue`]: crate::GraphQLValue
    fn as_bool(&self) -> Option<bool>;

    /// Converts this [`ScalarValue`] into another one.
    fn into_another<S: ScalarValue>(self) -> S {
        if let Some(i) = self.as_int() {
            S::from(i)
        } else if let Some(f) = self.as_float() {
            S::from(f)
        } else if let Some(b) = self.as_bool() {
            S::from(b)
        } else if let Some(s) = self.into_string() {
            S::from(s)
        } else {
            unreachable!("`ScalarValue` must represent at least one of the GraphQL spec types")
        }
    }
}

/// The default [`ScalarValue`] representation in [`juniper`].
///
/// These types closely follow the [GraphQL specification][0].
///
/// [0]: https://spec.graphql.org/October2021
#[derive(Clone, Debug, PartialEq, ScalarValue, Serialize)]
#[serde(untagged)]
pub enum DefaultScalarValue {
    /// [`Int` scalar][0] as a signed 32‐bit numeric non‐fractional value.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Int
    #[value(as_float, as_int)]
    Int(i32),

    /// [`Float` scalar][0] as a signed double‐precision fractional values as
    /// specified by [IEEE 754].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Float
    /// [IEEE 754]: https://en.wikipedia.org/wiki/IEEE_floating_point
    #[value(as_float)]
    Float(f64),

    /// [`String` scalar][0] as a textual data, represented as UTF‐8 character
    /// sequences.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-String
    #[value(as_str, as_string, into_string)]
    String(String),

    /// [`Boolean` scalar][0] as a `true` or `false` value.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Boolean
    #[value(as_bool)]
    Boolean(bool),
}

impl<'a> From<&'a str> for DefaultScalarValue {
    fn from(s: &'a str) -> Self {
        Self::String(s.into())
    }
}

impl<'a> From<Cow<'a, str>> for DefaultScalarValue {
    fn from(s: Cow<'a, str>) -> Self {
        Self::String(s.into())
    }
}
