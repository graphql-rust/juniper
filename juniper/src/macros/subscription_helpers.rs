use crate::{GraphQLType, ScalarValue, FieldError};
use futures::Stream;

/// Trait for converting  `T` to `Ok(T)` if T is not Result.
/// This is useful in subscription macros when user can provide type alias for
/// Stream or Result<Stream, _> and then a function on Stream should be called.
pub trait IntoFieldResult<T, S> {
    /// Turn current type into a generic result
    fn into_result(self) -> Result<T, FieldError<S>>;
}

impl<T, E, S> IntoFieldResult<T, S> for Result<T, E>
where E: Into<FieldError<S>> {
    fn into_result(self) -> Result<T, FieldError<S>> {
        self.map_err(|e| e.into())
    }
}

impl<T, I, S> IntoFieldResult<T, S> for T
where
    T: Stream<Item = I>,
{
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

/// This trait is used in `juniper::subscription` macro to get stream's
/// item type that implements `GraphQLType` from type alias provided
/// by user.
pub trait ExtractTypeFromStream<T, S>
where
    S: ScalarValue,
{
    /// Stream's return Value that will be returned if
    /// no errors occured. Is used to determine field type in
    /// `#[juniper::subscription]`
    type Item: GraphQLType<S>;
}

impl<T, I, S> ExtractTypeFromStream<StreamItem, S> for T
where
    T: futures::Stream<Item = I>,
    I: GraphQLType<S>,
    S: ScalarValue,
{
    type Item = I;
}

impl<Ty, T, E, S> ExtractTypeFromStream<StreamResult, S> for Ty
where
    Ty: futures::Stream<Item = Result<T, E>>,
    T: GraphQLType<S>,
    S: ScalarValue,
{
    type Item = T;
}

impl<T, I, E, S> ExtractTypeFromStream<ResultStreamItem, S> for Result<T, E>
where
    T: futures::Stream<Item = I>,
    I: GraphQLType<S>,
    S: ScalarValue,
{
    type Item = I;
}

impl<T, E, I, ER, S> ExtractTypeFromStream<ResultStreamResult, S> for Result<T, E>
where
    T: futures::Stream<Item = Result<I, ER>>,
    I: GraphQLType<S>,
    S: ScalarValue,
{
    type Item = I;
}
