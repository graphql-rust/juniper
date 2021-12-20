//! Helper traits and definitions for macros.

pub mod subscription;

use std::{fmt, rc::Rc, sync::Arc};

use futures::future::{self, BoxFuture};

use crate::{DefaultScalarValue, DynGraphQLValue, DynGraphQLValueAsync, FieldError, ScalarValue};

/// Conversion of a [`GraphQLValue`] to its [trait object][1].
///
/// [`GraphQLValue`]: crate::GraphQLValue
/// [1]: https://doc.rust-lang.org/reference/types/trait-object.html
pub trait AsDynGraphQLValue<S: ScalarValue = DefaultScalarValue> {
    /// Context type of this [`GraphQLValue`].
    ///
    /// [`GraphQLValue`]: crate::GraphQLValue
    type Context;

    /// Schema information type of this [`GraphQLValue`].
    ///
    /// [`GraphQLValue`]: crate::GraphQLValue
    type TypeInfo;

    /// Converts this value to a [`DynGraphQLValue`] [trait object][1].
    ///
    /// [1]: https://doc.rust-lang.org/reference/types/trait-object.html
    fn as_dyn_graphql_value(&self) -> &DynGraphQLValue<S, Self::Context, Self::TypeInfo>;

    /// Converts this value to a [`DynGraphQLValueAsync`] [trait object][1].
    ///
    /// [1]: https://doc.rust-lang.org/reference/types/trait-object.html
    fn as_dyn_graphql_value_async(&self)
        -> &DynGraphQLValueAsync<S, Self::Context, Self::TypeInfo>;
}

crate::sa::assert_obj_safe!(AsDynGraphQLValue<Context = (), TypeInfo = ()>);

/// This trait is used by [`graphql_scalar!`] macro to retrieve [`Error`] type
/// from a [`Result`].
///
/// [`Error`]: Result::Error
/// [`graphql_scalar!`]: macro@crate::graphql_scalar
pub trait ExtractError {
    /// Extracted [`Error`] type of this [`Result`].
    ///
    /// [`Error`]: Result::Error
    type Error;
}

impl<T, E> ExtractError for Result<T, E> {
    type Error = E;
}

/// Wraps `msg` with [`Display`] implementation into opaque [`Send`] [`Future`]
/// which immediately resolves into [`FieldError`].
pub fn err_fut<'ok, D, Ok, S>(msg: D) -> BoxFuture<'ok, Result<Ok, FieldError<S>>>
where
    D: fmt::Display,
    Ok: Send + 'ok,
    S: Send + 'static,
{
    Box::pin(future::err(FieldError::from(msg)))
}

/// Generates a [`FieldError`] for the given Rust type expecting to have
/// [`GraphQLType::name`].
///
/// [`GraphQLType::name`]: crate::GraphQLType::name
pub fn err_unnamed_type<S>(name: &str) -> FieldError<S> {
    FieldError::from(format!(
        "Expected `{}` type to implement `GraphQLType::name`",
        name,
    ))
}

/// Returns a [`future::err`] wrapping the [`err_unnamed_type`].
pub fn err_unnamed_type_fut<'ok, Ok, S>(name: &str) -> BoxFuture<'ok, Result<Ok, FieldError<S>>>
where
    Ok: Send + 'ok,
    S: Send + 'static,
{
    Box::pin(future::err(err_unnamed_type(name)))
}

/// Non-cryptographic hash with good dispersion to use [`str`](prim@str) in
/// const generics. See [spec] for more info.
///
/// [spec]: https://datatracker.ietf.org/doc/html/draft-eastlake-fnv-17.html
pub const fn fnv1a128(str: &str) -> u128 {
    const FNV_OFFSET_BASIS: u128 = 0x6c62272e07bb014262b821756295c58d;
    const FNV_PRIME: u128 = 0x0000000001000000000000000000013b;

    let bytes = str.as_bytes();
    let mut hash = FNV_OFFSET_BASIS;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u128;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

/// TODO
pub trait Type<S = DefaultScalarValue> {
    const NAME: &'static str;
}

impl<'a, S, T: Type<S> + ?Sized> Type<S> for &'a T {
    const NAME: &'static str = T::NAME;
}

impl<S, T: Type<S> + ?Sized> Type<S> for Box<T> {
    const NAME: &'static str = T::NAME;
}

impl<S, T: Type<S> + ?Sized> Type<S> for Arc<T> {
    const NAME: &'static str = T::NAME;
}

impl<S, T: Type<S> + ?Sized> Type<S> for Rc<T> {
    const NAME: &'static str = T::NAME;
}

/// TODO
pub trait SubTypes<S = DefaultScalarValue> {
    const NAMES: &'static [&'static str];
}

impl<'a, S, T: SubTypes<S> + ?Sized> SubTypes<S> for &'a T {
    const NAMES: &'static [&'static str] = T::NAMES;
}

impl<S, T: SubTypes<S> + ?Sized> SubTypes<S> for Box<T> {
    const NAMES: &'static [&'static str] = T::NAMES;
}

impl<S, T: SubTypes<S> + ?Sized> SubTypes<S> for Arc<T> {
    const NAMES: &'static [&'static str] = T::NAMES;
}

impl<S, T: SubTypes<S> + ?Sized> SubTypes<S> for Rc<T> {
    const NAMES: &'static [&'static str] = T::NAMES;
}
