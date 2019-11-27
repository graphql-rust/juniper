use crate::{DefaultScalarValue, GraphQLType, ScalarRefValue, ScalarValue};
use futures::Stream;
use std::convert::Infallible;

/// This trait is needed to convert
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

pub struct StreamItem;
pub struct StreamResult;
pub struct ResultStreamItem;
pub struct ResultStreamResult;

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
