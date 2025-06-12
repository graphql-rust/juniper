//! Helper traits and definitions for macros.

pub mod subscription;

use std::{convert::Infallible, fmt};

use futures::future::{self, BoxFuture};

use crate::{
    DefaultScalarValue, FieldError, FieldResult, GraphQLScalar, InputValue, IntoFieldError,
    ScalarValue,
};

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
        "Expected `{name}` type to implement `GraphQLType::name`",
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

/// [Autoref-based specialized][0] coercion into a [`Result`] for a function call for providing a
/// return-type polymorphism in macros.
///
/// [0]: https://lukaskalbertodt.github.io/2019/12/05/generalized-autoref-based-specialization.html
pub trait ToResultCall {
    /// Input of this function.
    type Input;
    /// Output of this function.
    type Output;
    /// Error of the [`Result`] coercion for this function.
    type Error;

    /// Calls this function, coercing its output into a [`Result`].
    fn __to_result_call(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;
}

impl<I, O> ToResultCall for fn(I) -> O {
    type Input = I;
    type Output = O;
    type Error = Infallible;

    fn __to_result_call(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        Ok(self(input))
    }
}

impl<I, O, E> ToResultCall for &fn(I) -> Result<O, E> {
    type Input = I;
    type Output = O;
    type Error = E;

    fn __to_result_call(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        self(input)
    }
}
