use crate::{DefaultScalarValue, GraphQLType, ScalarRefValue, ScalarValue};
use futures::Stream;
use std::convert::Infallible;

/// Trait for converting `T` to `Ok(T)` if T is not Result.
/// This is useful in subscription macros when user can provide type alias for
/// Stream or Result<Stream, _> and then a function on Stream should be called.
pub trait IntoResult<T, E> {
    fn into_result(self) -> Result<T, E>;
}

impl<T, E> IntoResult<T, E> for Result<T, E> {
    fn into_result(self) -> Result<T, E> {
        self
    }
}

impl<T, I> IntoResult<T, Infallible> for T
where
    T: Stream<Item = I>,
{
    fn into_result(self) -> Result<T, Infallible> {
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


/// This trait is used in `juniper::subscription` macro to get stream's
/// item type that implements `GraphQLType` from type alias provided
/// by user.
pub trait ExtractTypeFromStream<T, S>
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Item: GraphQLType<S>;
}

impl<T, I, S> ExtractTypeFromStream<StreamItem, S> for T
where
    T: futures::Stream<Item = I>,
    I: GraphQLType<S>,
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Item = I;
}

impl<Ty, T, E, S> ExtractTypeFromStream<StreamResult, S> for Ty
where
    Ty: futures::Stream<Item = Result<T, E>>,
    T: GraphQLType<S>,
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Item = T;
}

impl<T, I, E, S> ExtractTypeFromStream<ResultStreamItem, S> for Result<T, E>
where
    T: futures::Stream<Item = I>,
    I: GraphQLType<S>,
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Item = I;
}

impl<T, E, I, ER, S> ExtractTypeFromStream<ResultStreamResult, S> for Result<T, E>
where
    T: futures::Stream<Item = Result<I, ER>>,
    I: GraphQLType<S>,
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Item = I;
}
