//! Helper types for converting types to `Result<T, FieldError<E>>`.
//!
//! Used in `#[graphql_subscription]` macros to convert result type aliases on
//! subscription handlers to a concrete return type.

use futures::Stream;

use crate::{FieldError, GraphQLValue, IntoFieldError, ScalarValue};

/// Trait for wrapping [`Stream`] into [`Ok`] if it's not [`Result`].
///
/// Used in subscription macros when user can provide type alias for [`Stream`] or
/// `Result<Stream, _>` and then a function on [`Stream`] should be called.
pub trait IntoFieldResult<T, S> {
    /// Type of items yielded by this [`Stream`].
    type Item;

    /// Turns current [`Stream`] type into a generic [`Result`].
    fn into_result(self) -> Result<T, FieldError<S>>;
}

impl<T, E, S> IntoFieldResult<T, S> for Result<T, E>
where
    T: IntoFieldResult<T, S>,
    E: IntoFieldError<S>,
{
    type Item = T::Item;

    fn into_result(self) -> Result<T, FieldError<S>> {
        self.map_err(E::into_field_error)
    }
}

impl<T, S> IntoFieldResult<T, S> for T
where
    T: Stream,
{
    type Item = T::Item;

    fn into_result(self) -> Result<T, FieldError<S>> {
        Ok(self)
    }
}

/// This struct is used in `ExtractTypeFromStream` implementation for streams
/// of values.
pub struct StreamItem;

/// This struct is used in `ExtractTypeFromStream` implementation for results
/// with streams of values inside.
pub struct StreamResult;

/// This struct is used in `ExtractTypeFromStream` implementation for streams
/// of results of values inside.
pub struct ResultStreamItem;

/// This struct is used in `ExtractTypeFromStream` implementation for results
/// with streams of results of values inside.
pub struct ResultStreamResult;

/// This trait is used in `juniper::graphql_subscription` macro to get stream's
/// item type that implements `GraphQLValue` from type alias provided
/// by user.
pub trait ExtractTypeFromStream<T, S>
where
    S: ScalarValue,
{
    /// Stream's return Value that will be returned if
    /// no errors occured. Is used to determine field type in
    /// `#[juniper::graphql_subscription]`
    type Item: GraphQLValue<S>;
}

impl<T, I, S> ExtractTypeFromStream<StreamItem, S> for T
where
    T: futures::Stream<Item = I>,
    I: GraphQLValue<S>,
    S: ScalarValue,
{
    type Item = I;
}

impl<Ty, T, E, S> ExtractTypeFromStream<StreamResult, S> for Ty
where
    Ty: futures::Stream<Item = Result<T, E>>,
    T: GraphQLValue<S>,
    S: ScalarValue,
{
    type Item = T;
}

impl<T, I, E, S> ExtractTypeFromStream<ResultStreamItem, S> for Result<T, E>
where
    T: futures::Stream<Item = I>,
    I: GraphQLValue<S>,
    S: ScalarValue,
{
    type Item = I;
}

impl<T, E, I, ER, S> ExtractTypeFromStream<ResultStreamResult, S> for Result<T, E>
where
    T: futures::Stream<Item = Result<I, ER>>,
    I: GraphQLValue<S>,
    S: ScalarValue,
{
    type Item = I;
}
