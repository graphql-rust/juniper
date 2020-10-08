//! Helper types for converting types to `Result<T, FieldError<E>>`.
//!
//! Used in `#[graphql_subscription]` macros to convert result type aliases on
//! subscription handlers to a concrete return type.

use futures::Stream;

use crate::{FieldError, GraphQLValue};

/// Trait for wrapping [`Stream`] into [`Ok`] if it's not [`Result`].
///
/// Used in subscription macros when user can provide type alias for [`Stream`] or
/// `Result<Stream, _>` and then a function on [`Stream`] should be called.
pub trait IntoFieldResult<T> {
    /// Type of items yielded by this [`Stream`].
    type Item;

    /// Turns current [`Stream`] type into a generic [`Result`].
    fn into_result(self) -> Result<T, FieldError>;
}

impl<T, E> IntoFieldResult<T> for Result<T, E>
where
    T: IntoFieldResult<T>,
    E: Into<FieldError>,
{
    type Item = T::Item;

    fn into_result(self) -> Result<T, FieldError> {
        self.map_err(|e| e.into())
    }
}

impl<T> IntoFieldResult<T> for T
where
    T: Stream,
{
    type Item = T::Item;

    fn into_result(self) -> Result<T, FieldError> {
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
pub trait ExtractTypeFromStream<T> {
    /// Stream's return Value that will be returned if
    /// no errors occured. Is used to determine field type in
    /// `#[juniper::graphql_subscription]`
    type Item: GraphQLValue;
}

impl<T, I> ExtractTypeFromStream<StreamItem> for T
where
    T: futures::Stream<Item = I>,
    I: GraphQLValue,
{
    type Item = I;
}

impl<Ty, T, E> ExtractTypeFromStream<StreamResult> for Ty
where
    Ty: futures::Stream<Item = Result<T, E>>,
    T: GraphQLValue,
{
    type Item = T;
}

impl<T, I, E> ExtractTypeFromStream<ResultStreamItem> for Result<T, E>
where
    T: futures::Stream<Item = I>,
    I: GraphQLValue,
{
    type Item = I;
}

impl<T, E, I, ER> ExtractTypeFromStream<ResultStreamResult> for Result<T, E>
where
    T: futures::Stream<Item = Result<I, ER>>,
    I: GraphQLValue,
{
    type Item = I;
}
