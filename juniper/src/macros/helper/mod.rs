//! Helper traits and definitions for macros.

pub mod subscription;

use std::convert::Infallible;

use derive_more::with_trait::Display;
use futures::future::{self, BoxFuture};

use crate::{FieldError, InputValue, ScalarValue, ToScalarValue};

/// This trait is used by [`graphql_scalar`] macro to retrieve [`Error`] type from a [`Result`].
///
/// [`Error`]: Result::Error
/// [`graphql_scalar`]: macro@crate::graphql_scalar
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
    D: Display,
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

/// Error of an [`InputValue`] not representing a [`ScalarValue`], used in macro expansions.
#[derive(Display)]
#[display("Expected GraphQL scalar, found: {_0}")]
pub struct NotScalarError<'a, S: ScalarValue>(pub &'a InputValue<S>);

/// [Autoref-based specialized][0] coercion into a [`Result`] for a function call for providing a
/// return-type polymorphism in macros.
///
/// # Priority
///
/// 1. Functions returning [`Result`] are propagated "as is".
///
/// 2. Any other function's output is wrapped into [`Result`] with an [`Infallible`] [`Err`].
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

impl<I, O, E> ToResultCall for &fn(I) -> Result<O, E> {
    type Input = I;
    type Output = O;
    type Error = E;

    fn __to_result_call(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        self(input)
    }
}

impl<I, O> ToResultCall for fn(I) -> O {
    type Input = I;
    type Output = O;
    type Error = Infallible;

    fn __to_result_call(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        Ok(self(input))
    }
}

/// [Autoref-based specialized][0] coercion into a [`ScalarValue`] for a function call for providing
/// a return-type polymorphism in macros.
///
/// # Priority
///
/// 1. Functions returning a [`ScalarValue`] are propagated "as is".
///
/// 2. Functions returning a [`String`] are followed by [`From<String>`] conversion.
///
/// 3. Functions returning anything implementing [`ToScalarValue`] conversion are followed by this
///    conversion.
///
/// 4. Functions returning anything implementing [`Display`] are followed by the
///    [`ScalarValue::from_displayable_non_static()`] method call.
///
/// [0]: https://lukaskalbertodt.github.io/2019/12/05/generalized-autoref-based-specialization.html
pub trait ToScalarValueCall<S: ScalarValue> {
    /// Input of this function.
    type Input;

    /// Calls this function, coercing its output into a [`ScalarValue`].
    fn __to_scalar_value_call(&self, input: Self::Input) -> S;
}

impl<I, S> ToScalarValueCall<S> for &&&fn(I) -> S
where
    S: ScalarValue,
{
    type Input = I;

    fn __to_scalar_value_call(&self, input: Self::Input) -> S {
        self(input)
    }
}

impl<I, S> ToScalarValueCall<S> for &&fn(I) -> String
where
    S: ScalarValue,
{
    type Input = I;

    fn __to_scalar_value_call(&self, input: Self::Input) -> S {
        self(input).into()
    }
}

impl<I, O, S> ToScalarValueCall<S> for &fn(I) -> O
where
    S: ScalarValue,
    O: ToScalarValue<S>,
{
    type Input = I;

    fn __to_scalar_value_call(&self, input: Self::Input) -> S {
        self(input).to_scalar_value()
    }
}

impl<I, O, S> ToScalarValueCall<S> for fn(I) -> O
where
    S: ScalarValue,
    O: Display,
{
    type Input = I;

    fn __to_scalar_value_call(&self, input: Self::Input) -> S {
        S::from_displayable_non_static(&self(input))
    }
}
