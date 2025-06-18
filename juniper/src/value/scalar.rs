use std::convert::Infallible;

use arcstr::ArcStr;
use derive_more::with_trait::{Deref, Display, Error, From, TryInto};
use ref_cast::RefCast;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    any::{Any, TypeId},
    borrow::Cow,
    fmt, ptr,
};

use crate::{
    FieldError, IntoFieldError,
    parser::{ParseError, ScalarToken},
};
#[cfg(doc)]
use crate::{GraphQLScalar, GraphQLValue, Value};

pub use juniper_codegen::ScalarValue;

/// The result of converting a string into a scalar value
pub type ParseScalarResult<S = DefaultScalarValue> = Result<S, ParseError>;

/// A trait used to convert a `ScalarToken` into a certain scalar value type
pub trait ParseScalarValue<S = DefaultScalarValue> {
    /// See the trait documentation
    fn from_str(value: ScalarToken<'_>) -> ParseScalarResult<S>;
}

/// Type that could be used as internal representation of scalar values (e.g. inside [`Value`] and
/// [`InputValue`]).
///
/// This abstraction allows other libraries and user code to replace the default representation with
/// something that better fits their needs than [`DefaultScalarValue`].
///
/// # Deriving
///
/// There is a custom derive (`#[derive(`[`ScalarValue`](macro@crate::ScalarValue)`)]`) available,
/// that implements most of the required [`juniper`] traits automatically for an enum representing a
/// [`ScalarValue`]. However, [`Serialize`] and [`Deserialize`] implementations are expected to be
/// provided, as we as [`Display`], [`From`] and [`TryInto`] ones (for which it's convenient to use
/// [`derive_more`]).
///
/// # Example
///
/// The preferred way to define a new [`ScalarValue`] representation is defining an enum containing
/// a variant for each type that needs to be represented at the lowest level.
///
/// The following example introduces a new variant that is able to store 64-bit integers, and uses
/// a [`CompactString`] for a string representation.
///
/// ```rust
/// # use std::{any::Any, fmt};
/// #
/// # use compact_str::CompactString;
/// use derive_more::with_trait::{Display, From, TryInto};
/// use juniper::ScalarValue;
/// use serde::{de, Deserialize, Deserializer, Serialize};
///
/// #[derive(Clone, Debug, Display, From, PartialEq, ScalarValue, Serialize, TryInto)]
/// #[serde(untagged)]
/// #[value(from_displayable_with = from_compact_str)]
/// enum MyScalarValue {
///     #[from]
///     #[value(to_float, to_int)]
///     Int(i32),
///
///     #[from]
///     Long(i64),
///     
///     #[from]
///     #[value(to_float)]
///     Float(f64),
///
///     #[from(&str, String, CompactString)]
///     #[value(as_str, to_string)]
///     String(CompactString),
///     
///     #[from]
///     #[value(to_bool)]
///     Boolean(bool),
/// }
///
/// // Custom implementation of `ScalarValue::from_displayable()` method
/// // for efficient conversions from `CompactString` into `MyScalarValue`.
/// fn from_compact_str<Str: Display + Any + ?Sized>(s: &Str) -> MyScalarValue {
///     use juniper::AnyExt as _; // allows downcasting directly on types without `dyn`
///
///     if let Some(s) = s.downcast_ref::<CompactString>() {
///         MyScalarValue::String(s.clone())
///     } else {
///         s.to_string().into()
///     }
/// }
///
/// // `derive_more::TryInto` is not capable for transitive conversions yet,
/// // so this impl is manual as a custom string type is used instead of `String`.
/// impl TryFrom<MyScalarValue> for String {
///     type Error = MyScalarValue;
///
///     fn try_from(value: MyScalarValue) -> Result<Self, Self::Error> {
///         if let MyScalarValue::String(s) = value {
///             Ok(s.into())
///         } else {
///             Err(value)
///         }
///     }
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
///                 Ok(MyScalarValue::String(s.into()))
///             }
///         }
///
///         de.deserialize_any(Visitor)
///     }
/// }
/// ```
///
/// [`juniper`]: crate
/// [`CompactString`]: compact_str::CompactString
/// [`Deserialize`]: trait@serde::Deserialize
/// [`Serialize`]: trait@serde::Serialize
pub trait ScalarValue:
    fmt::Debug
    + Display
    + PartialEq
    + Clone
    + DeserializeOwned
    + Serialize
    + From<String>
    + From<bool>
    + From<i32>
    + From<f64>
    + for<'a> TryToPrimitive<'a, bool, Error: Display + IntoFieldError<Self>>
    + for<'a> TryToPrimitive<'a, i32, Error: Display + IntoFieldError<Self>>
    + for<'a> TryToPrimitive<'a, f64, Error: Display + IntoFieldError<Self>>
    + for<'a> TryToPrimitive<'a, String, Error: Display + IntoFieldError<Self>>
    + for<'a> TryToPrimitive<'a, &'a str, Error: Display + IntoFieldError<Self>>
    + TryInto<String>
    + 'static
{
    /// Checks whether this [`ScalarValue`] contains the value of the provided type `T`.
    ///
    /// # Implementation
    ///
    /// Implementations should implement this method.
    ///
    /// This is usually an enum dispatch with calling [`AnyExt::is::<T>()`] method on each variant.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use juniper::{ScalarValue as _, DefaultScalarValue};
    /// #
    /// let value = DefaultScalarValue::Int(42);
    ///
    /// assert_eq!(value.is_type::<i32>(), true);
    /// assert_eq!(value.is_type::<f64>(), false);
    /// ```
    #[must_use]
    fn is_type<T: Any + ?Sized>(&self) -> bool;

    /// Downcasts this [`ScalarValue`] as the value of the provided type `T`, if this
    /// [`ScalarValue`] represents the one.
    ///
    /// # Implementation
    ///
    /// Implementations should implement this method.
    ///
    /// This is usually an enum dispatch with calling [`AnyExt::downcast_ref::<T>()`] method on each
    /// variant.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use juniper::{ScalarValue as _, DefaultScalarValue};
    /// #
    /// let value = DefaultScalarValue::Int(42);
    ///
    /// assert_eq!(value.downcast_type::<i32>(), Some(&42));
    /// assert_eq!(value.downcast_type::<f64>(), None);
    /// ```
    ///
    /// # [`GraphQLScalar`] implementation
    ///
    /// This method is especially useful for performance, when a [`GraphQLScalar`] is implemented
    /// generically over a [`ScalarValue`], but based on the type that is very likely could be used
    /// in an optimized [`ScalarValue`] implementation.
    ///
    /// ```rust
    /// # use arcstr::ArcStr;
    /// # use juniper::{FieldResult, GraphQLScalar, Scalar, ScalarValue, Value};
    /// #
    /// #[derive(GraphQLScalar)]
    /// #[graphql(from_input_with = Self::from_input, transparent)]
    /// struct Name(ArcStr);
    ///
    /// impl Name {
    ///     fn from_input<S: ScalarValue>(v: &Scalar<S>) -> FieldResult<Self, S> {
    ///         // Check if our `ScalarValue` is represented by an `ArcStr` already, and if so,
    ///         // do the cheap `Clone` instead of allocating a new `ArcStr` in its `From<&str>`
    ///         // implementation.
    ///         let s = if let Some(s) = v.downcast_type::<ArcStr>() {
    ///             s.clone()
    ///         } else {
    ///             v.try_to::<&str>().map(ArcStr::from)?
    ///         };
    ///         if s.chars().next().is_some_and(char::is_uppercase) {
    ///             Ok(Self(s))
    ///         } else {
    ///             Err("`Name` should start with a capital letter".into())
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// However, this method is needed only when the type doesn't implement a [`GraphQLScalar`]
    /// itself, or does it in non-optimal way. In reality, the [`ArcStr`] already implements a
    /// [`GraphQLScalar`] and does the [`ScalarValue::downcast_type()`] check in its implementation,
    /// which can be naturally reused by calling the [`ScalarValue::try_to()`] method.
    ///
    /// ```rust
    /// # use arcstr::ArcStr;
    /// # use juniper::{FieldResult, GraphQLScalar, Scalar, ScalarValue, Value};
    /// #
    /// #[derive(GraphQLScalar)]
    /// #[graphql(from_input_with = Self::from_input, transparent)]
    /// struct Name(ArcStr);
    ///
    /// impl Name {
    ///     fn from_input(s: ArcStr) -> Result<Self, &'static str> {
    ///         //           ^^^^^^ macro expansion will call the `ScalarValue::try_to()` method
    ///         //                  to extract this type from the `ScalarValue` to this function
    ///         if s.chars().next().is_some_and(char::is_uppercase) {
    ///             Ok(Self(s))
    ///         } else {
    ///             Err("`Name` should start with a capital letter")
    ///         }
    ///     }
    /// }
    /// ```
    #[must_use]
    fn downcast_type<T: Any>(&self) -> Option<&T>;

    /// Tries to represent this [`ScalarValue`] as the specified type `T`.
    ///
    /// This method is the recommended way to parse a defined [`GraphQLScalar`] type `T` from a
    /// [`ScalarValue`].
    ///
    /// This method could be used instead of other `try_*` helpers in case the
    /// [`FromScalarValue::Error`] is needed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use juniper::{DefaultScalarValue, GraphQLScalar, ScalarValue as _};
    ///
    /// let v = DefaultScalarValue::Boolean(false);
    /// assert_eq!(v.try_to::<bool>().unwrap(), false);
    /// assert!(v.try_to::<f64>().is_err());
    ///
    /// #[derive(Debug, GraphQLScalar, PartialEq)]
    /// #[graphql(transparent)]
    /// struct Name(String);
    ///
    /// let v = DefaultScalarValue::String("John".into());
    /// assert_eq!(v.try_to::<String>().unwrap(), "John");
    /// assert_eq!(v.try_to::<&str>().unwrap(), "John");
    /// assert_eq!(v.try_to::<Name>().unwrap(), Name("John".into()));
    /// assert!(v.try_to::<i32>().is_err());
    /// ```
    ///
    /// # Implementation
    ///
    /// This method is an ergonomic alias for the [`FromScalarValue<T>`] conversion.
    ///
    /// Implementations should not implement this method, but rather implement only the
    /// [`TryToPrimitive<T>`] conversion directly in case `T` is a primitive built-in GraphQL
    /// scalar type ([`bool`], [`f64`], [`i32`], [`&str`], or [`String`]), otherwise the
    /// [`FromScalarValue<T>`] conversion is provided when a [`GraphQLScalar`] is implemented.
    fn try_to<'a, T>(&'a self) -> Result<T, T::Error>
    where
        T: FromScalarValue<'a, Self> + 'a,
    {
        T::from_scalar_value(self)
    }

    /// Tries to represent this [`ScalarValue`] as a [`bool`] value.
    ///
    /// Use the [`ScalarValue::try_to::<bool>()`] method in case the [`TryToPrimitive::Error`] is
    /// needed.
    ///
    /// # Implementation
    ///
    /// This method is an ergonomic alias for the [`TryToPrimitive<bool>`] conversion, which is used
    /// for implementing [`GraphQLValue`] for [`bool`] for all possible [`ScalarValue`]s.
    ///
    /// Implementations should not implement this method, but rather implement the
    /// [`TryToPrimitive<bool>`] conversions for all the supported boolean types.
    #[must_use]
    fn try_to_bool(&self) -> Option<bool> {
        self.try_to().ok()
    }

    /// Tries to represent this [`ScalarValue`] as an [`i32`] value.
    ///
    /// Use the [`ScalarValue::try_to::<i32>()`] method in case the [`TryToPrimitive::Error`] is
    /// needed.
    ///
    /// # Implementation
    ///
    /// This method is an ergonomic alias for the [`TryToPrimitive<i32>`] conversion, which is used
    /// for implementing [`GraphQLValue`] for [`i32`] for all possible [`ScalarValue`]s.
    ///
    /// Implementations should not implement this method, but rather implement the
    /// [`TryToPrimitive<i32>`] conversions for all the supported integer types with 32 bit or
    /// less to an integer, if requested.
    #[must_use]
    fn try_to_int(&self) -> Option<i32> {
        self.try_to().ok()
    }

    /// Tries to represent this [`ScalarValue`] as a [`f64`] value.
    ///
    /// Use the [`ScalarValue::try_to::<f64>()`] method in case the [`TryToPrimitive::Error`] is
    /// needed.
    ///
    /// # Implementation
    ///
    /// This method is an ergonomic alias for the [`TryToPrimitive<f64>`] conversion, which is used
    /// for implementing [`GraphQLValue`] for [`f64`] for all possible [`ScalarValue`]s.
    ///
    /// Implementations should not implement this method, but rather implement the
    /// [`TryToPrimitive<f64>`] conversions for all the supported integer types with 64 bit and
    /// all floating point values with 64 bit or less to a float, if requested.
    #[must_use]
    fn try_to_float(&self) -> Option<f64> {
        self.try_to().ok()
    }

    /// Tries to represent this [`ScalarValue`] as a [`String`] value.
    ///
    /// Allocates every time is called. For read-only and non-owning use of the underlying
    /// [`String`] value, consider using the [`ScalarValue::try_as_str()`] method.
    ///
    /// Use the [`ScalarValue::try_to::<String>()`] method in case the [`TryToPrimitive::Error`]
    /// is needed.
    ///
    /// # Implementation
    ///
    /// This method is an ergonomic alias for the [`TryToPrimitive<String>`] conversion, which is
    /// used for implementing [`GraphQLValue`] for [`String`] for all possible [`ScalarValue`]s.
    ///
    /// Implementations should not implement this method, but rather implement the
    /// [`TryToPrimitive<String>`] conversions for all the supported string types, if requested.
    #[must_use]
    fn try_to_string(&self) -> Option<String> {
        self.try_to().ok()
    }

    /// Tries to convert this [`ScalarValue`] into a [`String`] value.
    ///
    /// Similar to the [`ScalarValue::try_to_string()`] method, but takes ownership, so allows to
    /// omit redundant [`Clone`]ing.
    ///
    /// Use the [`TryInto<String>`] conversion in case the [`TryInto::Error`] is needed.
    ///
    /// # Implementation
    ///
    /// This method is an ergonomic alias for the [`TryInto<String>`] conversion.
    ///
    /// Implementations should not implement this method, but rather implement the
    /// [`TryInto<String>`] conversion for all the supported string types, if requested.
    #[must_use]
    fn try_into_string(self) -> Option<String> {
        self.try_into().ok()
    }

    /// Tries to represent this [`ScalarValue`] as a [`str`] value.
    ///
    /// Use the [`ScalarValue::try_to::<&str>()`] method in case the [`TryToPrimitive::Error`]
    /// is needed.
    ///
    /// # Implementation
    ///
    /// This method is an ergonomic alias for the [`TryToPrimitive`]`<&`[`str`]`>` conversion, which
    /// is used for implementing [`GraphQLValue`] for [`String`] for all possible [`ScalarValue`]s.
    ///
    /// Implementations should not implement this method, but rather implement the
    /// [`TryToPrimitive`]`<&`[`str`]`>` conversions for all the supported string types, if
    /// requested.
    #[must_use]
    fn try_as_str(&self) -> Option<&str> {
        self.try_to().ok()
    }

    /// Converts this [`ScalarValue`] into another one via [`i32`], [`f64`], [`bool`] or [`String`]
    /// conversion.
    ///
    /// # Panics
    ///
    /// If this [`ScalarValue`] doesn't represent at least one of [`i32`], [`f64`], [`bool`] or
    /// [`String`].
    #[must_use]
    fn into_another<S: ScalarValue>(self) -> S {
        if let Some(i) = self.try_to_int() {
            S::from(i)
        } else if let Some(f) = self.try_to_float() {
            S::from(f)
        } else if let Some(b) = self.try_to_bool() {
            S::from(b)
        } else if let Some(s) = self.try_into_string() {
            S::from(s)
        } else {
            unreachable!("`ScalarValue` must represent at least one of the GraphQL spec types")
        }
    }

    /// Creates this [`ScalarValue`] from the provided [`Display`]able type.
    ///
    /// This method should be implemented if [`ScalarValue`] implementation uses some custom string
    /// type inside to enable efficient conversion from values of this type.
    ///
    /// Default implementation allocates by converting [`ToString`] and [`From<String>`].
    ///
    /// # Example
    ///
    /// See the [example in trait documentation](ScalarValue#example) for how it can be used.
    #[must_use]
    fn from_displayable<Str: Display + Any + ?Sized>(s: &Str) -> Self {
        s.to_string().into()
    }
}

/// Fallible representation of a [`ScalarValue`] as one of the types it consists of, or derived ones
/// from them.
///
/// # Implementation
///
/// Implementing this trait for a type allows to specify this type directly in the `from_input()`
/// function when implementing a [`GraphQLScalar`] via [derive macro](macro@GraphQLScalar).
///
/// `#[derive(`[`ScalarValue`](macro@crate::ScalarValue)`)]` automatically implements this trait for
/// all the required primitive types if `#[to_<type>]` and `#[as_<type>]` attributes are specified.
pub trait TryToPrimitive<'me, T: 'me> {
    /// Error if this [`ScalarValue`] doesn't represent the expected type.
    type Error: 'me;

    /// Tries to represent this [`ScalarValue`] as the expected type.
    ///
    /// # Errors
    ///
    /// If this [`ScalarValue`] doesn't represent the expected type.
    fn try_to_primitive(&'me self) -> Result<T, Self::Error>;
}

/// Parsing of a [`ScalarValue`] into a Rust data type.
///
/// The conversion _can_ fail, and must in that case return an [`Err`].
///
/// Use the [`ScalarValue::try_to()`] method as a shortcut for this conversion.
///
/// # Implementation
///
/// Implementing this trait for a type allows to specify this type directly in the `from_input()`
/// function when implementing a [`GraphQLScalar`] via [derive macro](macro@GraphQLScalar).
///
/// Also, `#[derive(`[`GraphQLScalar`](macro@GraphQLScalar)`)]` automatically implements this trait
/// for a type.
pub trait FromScalarValue<'s, S: 's = DefaultScalarValue>: Sized {
    /// Parsing error of a [`ScalarValue`].
    type Error: IntoFieldError<S> + 's;

    /// Parses the provided [`ScalarValue`].
    ///
    /// # Errors
    ///
    /// If this type cannot be parsed from the provided [`ScalarValue`].
    fn from_scalar_value(v: &'s S) -> Result<Self, Self::Error>;
}

impl<'s, S> FromScalarValue<'s, S> for &'s S {
    type Error = Infallible;

    fn from_scalar_value(v: &'s S) -> Result<Self, Self::Error> {
        Ok(v)
    }
}

impl<'s, S: ScalarValue> FromScalarValue<'s, S> for &'s Scalar<S> {
    type Error = Infallible;

    fn from_scalar_value(v: &'s S) -> Result<Self, Self::Error> {
        Ok(v.into())
    }
}

/// Error of a [`ScalarValue`] not matching the expected type.
#[derive(Clone, Debug, Display, Error)]
#[display("Expected `{type_name}`, found: {}", <&Scalar<_>>::from(*input))]
pub struct WrongInputScalarTypeError<'a, S: ScalarValue> {
    /// Type name of the expected GraphQL scalar.
    pub type_name: ArcStr,

    /// Input [`ScalarValue`] not matching the expected type.
    pub input: &'a S,
}

impl<'a, S: ScalarValue> IntoFieldError<S> for WrongInputScalarTypeError<'a, S> {
    fn into_field_error(self) -> FieldError<S> {
        FieldError::<S>::from(self)
    }
}

pub trait ToScalarValue<S = DefaultScalarValue> {
    /// Converts this value into a [`ScalarValue`].
    #[must_use]
    fn to_scalar_value(&self) -> S;
}

/// Transparent wrapper over a value, indicating it being a [`ScalarValue`].
///
/// Used in [`GraphQLScalar`] definitions to distinguish a concrete type for a generic
/// [`ScalarValue`], since Rust type inference fail do so for a generic value directly in macro
/// expansions.
#[derive(Debug, Deref, RefCast)]
#[repr(transparent)]
pub struct Scalar<T: ScalarValue>(T);

impl<'a, T: ScalarValue> From<&'a T> for &'a Scalar<T> {
    fn from(value: &'a T) -> Self {
        Scalar::ref_cast(value)
    }
}

impl<S: ScalarValue> Display for Scalar<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(s) = self.0.try_as_str() {
            write!(f, "\"{s}\"")
        } else {
            Display::fmt(&self.0, f)
        }
    }
}

/// Extension of [`Any`] for using its methods directly on the value without `dyn`.
pub trait AnyExt: Any {
    /// Returns `true` if the this type is the same as `T`.
    #[must_use]
    fn is<T: Any + ?Sized>(&self) -> bool {
        TypeId::of::<T>() == self.type_id()
    }

    /// Returns [`Some`] reference to this value if it's of type `T`, or [`None`] otherwise.
    #[must_use]
    fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.is::<T>()
            .then(|| unsafe { &*(ptr::from_ref(self) as *const T) })
    }

    /// Returns [`Some`] mutable reference to this value if it's of type `T`, or [`None`] otherwise.
    #[must_use]
    fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        // `self.is::<T>()` produces a false positive here: borrowed data escapes outside of method
        (TypeId::of::<Self>() == TypeId::of::<T>())
            .then(|| unsafe { &mut *(ptr::from_mut(self) as *mut T) })
    }
}

impl<T: Any + ?Sized> AnyExt for T {}

/// The default [`ScalarValue`] representation in [`juniper`].
///
/// These types closely follow the [GraphQL specification][0].
///
/// [0]: https://spec.graphql.org/October2021
#[derive(Clone, Debug, Display, From, PartialEq, ScalarValue, Serialize, TryInto)]
#[serde(untagged)]
pub enum DefaultScalarValue {
    /// [`Int` scalar][0] as a signed 32‐bit numeric non‐fractional value.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Int
    #[from]
    #[value(to_float, to_int)]
    Int(i32),

    /// [`Float` scalar][0] as a signed double‐precision fractional values as
    /// specified by [IEEE 754].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Float
    /// [IEEE 754]: https://en.wikipedia.org/wiki/IEEE_floating_point
    #[from]
    #[value(to_float)]
    Float(f64),

    /// [`String` scalar][0] as a textual data, represented as UTF‐8 character
    /// sequences.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-String
    #[from(&str, Cow<'_, str>, String)]
    #[value(as_str, to_string)]
    String(String),

    /// [`Boolean` scalar][0] as a `true` or `false` value.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Boolean
    #[from]
    #[value(to_bool)]
    Boolean(bool),
}
