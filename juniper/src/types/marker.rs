//! Marker traits for GraphQL types.
//!
//! This module provide specialized types for GraphQL. To ensure that
//! only specification compliant construct compile, these marker
//! traits are used. Encountering an error where one of these traits
//! is involved implies that the construct is not valid in GraphQL.

use crate::{GraphQLType, ScalarValue};

/// Maker object for GraphQL objects.
///
/// This trait extends the GraphQLType and is only used to mark
/// object. During compile this addition information is required to
/// prevent unwanted structure compiling. If an object requires this
/// trait instead of the GraphQLType, then it explicitly requires an
/// GraphQL objects. Other types (scalars, enums, and input objects)
/// are not allowed.
pub trait GraphQLObjectType<S: ScalarValue>: GraphQLType<S> {
    /// An arbitrary function without meaning.
    ///
    /// May contain compile timed check logic which ensures that types
    /// are used correctly according to the GraphQL specification.
    fn mark() {}
}

/// Marker trait for types which can be used as output types.
///
/// The GraphQL specification differentiates between input and output
/// types. Each type which can be used as an output type should
/// implement this trait. The specification defines enum, scalar,
/// object, union, and interface as output types.
pub trait IsOutputType<S: ScalarValue>: GraphQLType<S> {
    /// An arbitrary function without meaning.
    ///
    /// May contain compile timed check logic which ensures that types
    /// are used correctly according to the GraphQL specification.
    fn mark() {}
}

/// Marker trait for types which can be used as input types.
///
/// The GraphQL specification differentiates between input and output
/// types. Each type which can be used as an input type should
/// implement this trait. The specification defines enum, scalar, and
/// input object input types.
pub trait IsInputType<S: ScalarValue>: GraphQLType<S> {
    /// An arbitrary function without meaning.
    ///
    /// May contain compile timed check logic which ensures that types
    /// are used correctly according to the GraphQL specification.
    fn mark() {}
}

impl<S, T> IsInputType<S> for Option<T>
where
    T: IsInputType<S> + Send + Sync,
    S: ScalarValue,
{
}

impl<S, T> IsOutputType<S> for Option<T>
where
    T: IsOutputType<S> + Send + Sync,
    S: ScalarValue,
{
}

impl<S, T> IsOutputType<S> for Vec<T>
where
    T: IsOutputType<S> + Send + Sync,
    S: ScalarValue,
{
}

impl<'a, S, T> IsOutputType<S> for &'a [T]
where
    T: IsOutputType<S> + Send + Sync,
    S: ScalarValue,
{
}

impl<S, T> IsInputType<S> for Vec<T>
where
    T: IsInputType<S> + Send + Sync,
    S: ScalarValue,
{
}

impl<'a, S, T> IsInputType<S> for &'a [T]
where
    T: IsInputType<S> + Send + Sync,
    S: ScalarValue,
{
}

impl<'a, S, T> IsInputType<S> for &T
where
    T: IsInputType<S> + Send + Sync,
    S: ScalarValue,
{
}
impl<'a, S, T> IsOutputType<S> for &T
where
    T: IsOutputType<S> + Send + Sync,
    S: ScalarValue,
{
}

impl<S, T> IsInputType<S> for Box<T>
where
    T: IsInputType<S> + Send + Sync,
    S: ScalarValue,
{
}
impl<S, T> IsOutputType<S> for Box<T>
where
    T: IsOutputType<S> + Send + Sync,
    S: ScalarValue,
{
}

impl<'a, S> IsInputType<S> for &str where S: ScalarValue {}
impl<'a, S> IsOutputType<S> for &str where S: ScalarValue {}
