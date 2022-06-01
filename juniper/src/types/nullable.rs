//! GraphQL implementation for [`Nullable`].

use std::mem;

use futures::future;

use crate::{
    ast::{FromInputValue, InputValue, ToInputValue},
    executor::{ExecutionResult, Executor, Registry},
    graphql, reflect, resolve,
    schema::meta::MetaType,
    types::{
        async_await::GraphQLValueAsync,
        base::{GraphQLType, GraphQLValue},
        marker::IsInputType,
    },
    BoxFuture, ScalarValue, Selection,
};

/// [`Nullable`] wrapper allowing to distinguish between an implicit and
/// explicit `null` input value.
///
/// [GraphQL spec states][0] that these two field calls are similar, but are not
/// identical:
///
/// > ```graphql
/// > {
/// >   field(arg: null)
/// >   field
/// > }
/// > ```
/// > The first has explicitly provided `null` to the argument "arg", while the
/// > second has implicitly not provided a value to the argument "arg". These
/// > two forms may be interpreted differently. For example, a mutation
/// > representing deleting a field vs not altering a field, respectively.
///
/// In cases where there is no need to distinguish between the two types of
/// `null`, it's better to simply use [`Option`].
///
/// [0]: https://spec.graphql.org/October2021#example-1c7eb
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Nullable<T> {
    /// No value specified.
    ImplicitNull,

    /// Value explicitly specified to be `null`.
    ExplicitNull,

    /// Explicitly specified non-`null` value of `T`.
    Some(T),
}

impl<T> Default for Nullable<T> {
    fn default() -> Self {
        Self::ImplicitNull
    }
}

impl<T> Nullable<T> {
    /// Indicates whether this [`Nullable`] represents an [`ExplicitNull`].
    ///
    /// [`ExplicitNull`]: Nullable::ExplicitNull
    #[inline]
    pub fn is_explicit_null(&self) -> bool {
        matches!(self, Self::ExplicitNull)
    }

    /// Indicates whether this [`Nullable`] represents an [`ImplicitNull`].
    ///
    /// [`ImplicitNull`]: Nullable::ImplicitNull
    #[inline]
    pub fn is_implicit_null(&self) -> bool {
        matches!(self, Self::ImplicitNull)
    }

    /// Indicates whether this [`Nullable`] contains a non-`null` value.
    #[inline]
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    /// Indicates whether this [`Nullable`] represents a `null`.
    #[inline]
    pub fn is_null(&self) -> bool {
        !matches!(self, Self::Some(_))
    }

    /// Converts from `&Nullable<T>` to `Nullable<&T>`.
    #[inline]
    pub fn as_ref(&self) -> Nullable<&T> {
        match self {
            Self::Some(x) => Nullable::Some(x),
            Self::ImplicitNull => Nullable::ImplicitNull,
            Self::ExplicitNull => Nullable::ExplicitNull,
        }
    }

    /// Converts from `&mut Nullable<T>` to `Nullable<&mut T>`.
    #[inline]
    pub fn as_mut(&mut self) -> Nullable<&mut T> {
        match self {
            Self::Some(x) => Nullable::Some(x),
            Self::ImplicitNull => Nullable::ImplicitNull,
            Self::ExplicitNull => Nullable::ExplicitNull,
        }
    }

    /// Returns the contained non-`null` value, consuming the `self` value.
    ///
    /// # Panics
    ///
    /// With a custom `msg` if this [`Nullable`] represents a `null`.
    #[inline]
    #[track_caller]
    pub fn expect(self, msg: &str) -> T {
        self.some().expect(msg)
    }

    /// Returns the contained non-`null` value or the provided `default` one.
    #[inline]
    pub fn unwrap_or(self, default: T) -> T {
        self.some().unwrap_or(default)
    }

    /// Returns thecontained non-`null` value  or computes it from the provided
    /// `func`tion.
    #[inline]
    pub fn unwrap_or_else<F: FnOnce() -> T>(self, func: F) -> T {
        self.some().unwrap_or_else(func)
    }

    /// Returns the contained non-`null` value or the [`Default`] one.
    #[inline]
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        self.some().unwrap_or_default()
    }

    /// Maps this `Nullable<T>` to `Nullable<U>` by applying the provided
    /// `func`tion to the contained non-`null` value.
    #[inline]
    pub fn map<U, F: FnOnce(T) -> U>(self, func: F) -> Nullable<U> {
        match self {
            Self::Some(x) => Nullable::Some(func(x)),
            Self::ImplicitNull => Nullable::ImplicitNull,
            Self::ExplicitNull => Nullable::ExplicitNull,
        }
    }

    /// Applies the provided `func`tion to the contained non-`null` value (if
    /// any), or returns the provided `default` value (if not).
    #[inline]
    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, func: F) -> U {
        self.some().map_or(default, func)
    }

    /// Applies the provided `func`tion to the contained non-`null` value (if
    /// any), or computes the provided `default` one (if not).
    #[inline]
    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, func: F) -> U {
        self.some().map_or_else(default, func)
    }

    /// Transforms this `Nullable<T>` into a `Result<T, E>`, mapping `Some(v)`
    /// to `Ok(v)` and [`ImplicitNull`] or [`ExplicitNull`] to `Err(err)`.
    ///
    /// [`ExplicitNull`]: Nullable::ExplicitNull
    /// [`ImplicitNull`]: Nullable::ImplicitNull
    #[inline]
    pub fn ok_or<E>(self, err: E) -> Result<T, E> {
        self.some().ok_or(err)
    }

    /// Transforms this `Nullable<T>` into a `Result<T, E>`, mapping `Some(v)`
    /// to `Ok(v)` and [`ImplicitNull`] or [`ExplicitNull`] to `Err(err())`.
    ///
    /// [`ExplicitNull`]: Nullable::ExplicitNull
    /// [`ImplicitNull`]: Nullable::ImplicitNull
    #[inline]
    pub fn ok_or_else<E, F: FnOnce() -> E>(self, err: F) -> Result<T, E> {
        self.some().ok_or_else(err)
    }

    /// Returns this [`Nullable`] if it contains a non-`null` value, otherwise
    /// returns the specified `b` [`Nullable`] value.
    #[inline]
    #[must_use]
    pub fn or(self, b: Self) -> Self {
        match self {
            Self::Some(_) => self,
            _ => b,
        }
    }

    /// Returns this [`Nullable`] if it contains a non-`null` value, otherwise
    /// computes a [`Nullable`] value from the specified `func`tion.
    #[inline]
    #[must_use]
    pub fn or_else<F: FnOnce() -> Nullable<T>>(self, func: F) -> Nullable<T> {
        match self {
            Self::Some(_) => self,
            _ => func(),
        }
    }

    /// Replaces the contained non-`null` value in this [`Nullable`] by the
    /// provided `value`, returning the old one if present, leaving a [`Some`]
    /// in its place without deinitializing either one.
    ///
    /// [`Some`]: Nullable::Some
    #[inline]
    #[must_use]
    pub fn replace(&mut self, value: T) -> Self {
        mem::replace(self, Self::Some(value))
    }

    /// Converts this [`Nullable`] to [Option].
    #[inline]
    pub fn some(self) -> Option<T> {
        match self {
            Self::Some(v) => Some(v),
            Self::ExplicitNull | Self::ImplicitNull => None,
        }
    }

    /// Converts this [`Nullable`] to `Option<Option<T>>`, mapping `Some(v)` to
    /// `Some(Some(v))`, [`ExplicitNull`] to `Some(None)`, and [`ImplicitNull`]
    /// to [`None`].
    ///
    /// [`ExplicitNull`]: Nullable::ExplicitNull
    /// [`ImplicitNull`]: Nullable::ImplicitNull
    pub fn explicit(self) -> Option<Option<T>> {
        match self {
            Self::Some(v) => Some(Some(v)),
            Self::ExplicitNull => Some(None),
            Self::ImplicitNull => None,
        }
    }
}

impl<T: Copy> Nullable<&T> {
    /// Maps this `Nullable<&T>` to a `Nullable<T>` by [`Copy`]ing the contents
    /// of this [`Nullable`].
    pub fn copied(self) -> Nullable<T> {
        self.map(|&t| t)
    }
}

impl<T: Copy> Nullable<&mut T> {
    /// Maps this `Nullable<&mut T>` to a `Nullable<T>` by [`Copy`]ing the
    /// contents of this [`Nullable`].
    pub fn copied(self) -> Nullable<T> {
        self.map(|&mut t| t)
    }
}

impl<T: Clone> Nullable<&T> {
    /// Maps this `Nullable<&T>` to a `Nullable<T>` by [`Clone`]ing the contents
    /// of this [`Nullable`].
    pub fn cloned(self) -> Nullable<T> {
        self.map(|t| t.clone())
    }
}

impl<T: Clone> Nullable<&mut T> {
    /// Maps this `Nullable<&mut T>` to a `Nullable<T>` by [`Clone`]ing the
    /// contents of this [`Nullable`].
    pub fn cloned(self) -> Nullable<T> {
        self.map(|t| t.clone())
    }
}

impl<T, Info, S> resolve::Type<Info, S> for Nullable<T>
where
    T: resolve::Type<Info, S>,
    Info: ?Sized,
{
    fn meta<'r>(registry: &mut Registry<'r, S>, info: &Info) -> MetaType<'r, S>
    where
        S: 'r,
    {
        registry.build_nullable_type_new::<T, _>(info).into_meta()
    }
}

impl<T, Info, Ctx, S> resolve::Value<Info, Ctx, S> for Nullable<T>
where
    T: resolve::Value<Info, Ctx, S>,
    Info: ?Sized,
    Ctx: ?Sized,
{
    fn resolve_value(
        &self,
        selection_set: Option<&[Selection<'_, S>]>,
        info: &Info,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S> {
        match self {
            Self::Some(v) => v.resolve_value(selection_set, info, executor),
            Self::ExplicitNull | Self::ImplicitNull => Ok(graphql::Value::Null),
        }
    }
}

impl<T, Info, Ctx, S> resolve::ValueAsync<Info, Ctx, S> for Nullable<T>
where
    T: resolve::ValueAsync<Info, Ctx, S>,
    Info: ?Sized,
    Ctx: ?Sized,
    S: Send,
{
    fn resolve_value_async<'r>(
        &'r self,
        selection_set: Option<&'r [Selection<'_, S>]>,
        info: &'r Info,
        executor: &'r Executor<Ctx, S>,
    ) -> BoxFuture<'r, ExecutionResult<S>> {
        match self {
            Self::Some(v) => v.resolve_value_async(selection_set, info, executor),
            Self::ExplicitNull | Self::ImplicitNull => Box::pin(future::ok(graphql::Value::Null)),
        }
    }
}

impl<T, S> resolve::ToInputValue<S> for Nullable<T>
where
    T: resolve::ToInputValue<S>,
{
    fn to_input_value(&self) -> graphql::InputValue<S> {
        match self {
            Self::Some(v) => v.to_input_value(),
            Self::ExplicitNull | Self::ImplicitNull => graphql::InputValue::Null,
        }
    }
}

impl<'inp, T, S: 'inp> resolve::InputValue<'inp, S> for Nullable<T>
where
    T: resolve::InputValue<'inp, S>,
{
    type Error = <T as resolve::InputValue<'inp, S>>::Error;

    fn try_from_input_value(v: &'inp InputValue<S>) -> Result<Self, Self::Error> {
        if v.is_null() {
            Ok(Self::ExplicitNull)
        } else {
            <T as resolve::InputValue<'inp, S>>::try_from_input_value(v).map(Self::Some)
        }
    }

    fn try_from_implicit_null() -> Result<Self, Self::Error> {
        Ok(Self::ImplicitNull)
    }
}

impl<T, S> graphql::InputType<S> for Nullable<T>
where
    T: graphql::InputType<S>,
{
    fn assert_input_type() {
        T::assert_input_type()
    }
}

impl<T, S> graphql::OutputType<S> for Nullable<T>
where
    T: graphql::OutputType<S>,
{
    fn assert_output_type() {
        T::assert_output_type()
    }
}

impl<T, S> reflect::BaseType<S> for Nullable<T>
where
    T: reflect::BaseType<S>,
{
    const NAME: reflect::Type = T::NAME;
}

impl<T, S> reflect::BaseSubTypes<S> for Nullable<T>
where
    T: reflect::BaseSubTypes<S>,
{
    const NAMES: reflect::Types = T::NAMES;
}

impl<T, S> reflect::WrappedType<S> for Nullable<T>
where
    T: reflect::WrappedType<S>,
{
    const VALUE: reflect::WrappedValue = reflect::wrap::nullable(T::VALUE);
}

////////////////////////////////////////////////////////////////////////////////

impl<S, T> GraphQLType<S> for Nullable<T>
where
    T: GraphQLType<S>,
    S: ScalarValue,
{
    fn name(_: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        registry.build_nullable_type::<T>(info).into_meta()
    }
}

impl<S, T> GraphQLValue<S> for Nullable<T>
where
    S: ScalarValue,
    T: GraphQLValue<S>,
{
    type Context = T::Context;
    type TypeInfo = T::TypeInfo;

    fn type_name(&self, _: &Self::TypeInfo) -> Option<&'static str> {
        None
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        _: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        match *self {
            Self::Some(ref obj) => executor.resolve(info, obj),
            _ => Ok(graphql::Value::null()),
        }
    }
}

impl<S, T> GraphQLValueAsync<S> for Nullable<T>
where
    T: GraphQLValueAsync<S>,
    T::TypeInfo: Sync,
    T::Context: Sync,
    S: ScalarValue + Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        _: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        let f = async move {
            let value = match self {
                Self::Some(obj) => executor.resolve_into_value_async(info, obj).await,
                _ => graphql::Value::null(),
            };
            Ok(value)
        };
        Box::pin(f)
    }
}

impl<S, T: FromInputValue<S>> FromInputValue<S> for Nullable<T> {
    type Error = <T as FromInputValue<S>>::Error;

    fn from_input_value(v: &InputValue<S>) -> Result<Self, Self::Error> {
        match v {
            &InputValue::Null => Ok(Self::ExplicitNull),
            v => v.convert().map(Self::Some),
        }
    }

    fn from_implicit_null() -> Result<Self, Self::Error> {
        Ok(Self::ImplicitNull)
    }
}

impl<S, T> ToInputValue<S> for Nullable<T>
where
    T: ToInputValue<S>,
    S: ScalarValue,
{
    fn to_input_value(&self) -> InputValue<S> {
        match *self {
            Self::Some(ref v) => v.to_input_value(),
            _ => InputValue::null(),
        }
    }
}

impl<S, T> IsInputType<S> for Nullable<T>
where
    T: IsInputType<S>,
    S: ScalarValue,
{
}
