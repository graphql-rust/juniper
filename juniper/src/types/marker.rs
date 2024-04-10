//! Marker traits for GraphQL types.
//!
//! This module provide specialized types for GraphQL. To ensure that
//! only specification compliant construct compile, these marker
//! traits are used. Encountering an error where one of these traits
//! is involved implies that the construct is not valid in GraphQL.

use std::sync::Arc;

use crate::{GraphQLType, ScalarValue};

/// Maker trait for [GraphQL objects][1].
///
/// This trait extends the [`GraphQLType`] and is only used to mark an [object][1]. During
/// compile this addition information is required to prevent unwanted structure compiling. If an
/// object requires this trait instead of the [`GraphQLType`], then it explicitly requires
/// [GraphQL objects][1]. Other types ([scalars][2], [enums][3], [interfaces][4], [input objects][5]
/// and [unions][6]) are not allowed.
///
/// [1]: https://spec.graphql.org/October2021#sec-Objects
/// [2]: https://spec.graphql.org/October2021#sec-Scalars
/// [3]: https://spec.graphql.org/October2021#sec-Enums
/// [4]: https://spec.graphql.org/October2021#sec-Interfaces
/// [5]: https://spec.graphql.org/October2021#sec-Input-Objects
/// [6]: https://spec.graphql.org/October2021#sec-Unions
pub trait GraphQLObject<S: ScalarValue>: GraphQLType<S> {
    /// An arbitrary function without meaning.
    ///
    /// May contain compile timed check logic which ensures that types are used correctly according
    /// to the [GraphQL specification][1].
    ///
    /// [1]: https://spec.graphql.org/October2021
    fn mark() {}
}

impl<S, T> GraphQLObject<S> for &T
where
    T: GraphQLObject<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> GraphQLObject<S> for Box<T>
where
    T: GraphQLObject<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> GraphQLObject<S> for Arc<T>
where
    T: GraphQLObject<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

/// Maker trait for [GraphQL interfaces][1].
///
/// This trait extends the [`GraphQLType`] and is only used to mark an [interface][1]. During
/// compile this addition information is required to prevent unwanted structure compiling. If an
/// object requires this trait instead of the [`GraphQLType`], then it explicitly requires
/// [GraphQL interfaces][1]. Other types ([scalars][2], [enums][3], [objects][4], [input objects][5]
/// and [unions][6]) are not allowed.
///
/// [1]: https://spec.graphql.org/October2021#sec-Interfaces
/// [2]: https://spec.graphql.org/October2021#sec-Scalars
/// [3]: https://spec.graphql.org/October2021#sec-Enums
/// [4]: https://spec.graphql.org/October2021#sec-Objects
/// [5]: https://spec.graphql.org/October2021#sec-Input-Objects
/// [6]: https://spec.graphql.org/October2021#sec-Unions
pub trait GraphQLInterface<S: ScalarValue>: GraphQLType<S> {
    /// An arbitrary function without meaning.
    ///
    /// May contain compile timed check logic which ensures that types are used correctly according
    /// to the [GraphQL specification][1].
    ///
    /// [1]: https://spec.graphql.org/October2021
    fn mark() {}
}

impl<S, T> GraphQLInterface<S> for &T
where
    T: GraphQLInterface<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> GraphQLInterface<S> for Box<T>
where
    T: GraphQLInterface<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> GraphQLInterface<S> for Arc<T>
where
    T: GraphQLInterface<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

/// Maker trait for [GraphQL unions][1].
///
/// This trait extends the [`GraphQLType`] and is only used to mark an [union][1]. During compile
/// this addition information is required to prevent unwanted structure compiling. If an object
/// requires this trait instead of the [`GraphQLType`], then it explicitly requires
/// [GraphQL unions][1]. Other types ([scalars][2], [enums][3], [objects][4], [input objects][5] and
/// [interfaces][6]) are not allowed.
///
/// [1]: https://spec.graphql.org/October2021#sec-Unions
/// [2]: https://spec.graphql.org/October2021#sec-Scalars
/// [3]: https://spec.graphql.org/October2021#sec-Enums
/// [4]: https://spec.graphql.org/October2021#sec-Objects
/// [5]: https://spec.graphql.org/October2021#sec-Input-Objects
/// [6]: https://spec.graphql.org/October2021#sec-Interfaces
pub trait GraphQLUnion<S: ScalarValue>: GraphQLType<S> {
    /// An arbitrary function without meaning.
    ///
    /// May contain compile timed check logic which ensures that types are used correctly according
    /// to the [GraphQL specification][1].
    ///
    /// [1]: https://spec.graphql.org/October2021
    fn mark() {}
}

impl<S, T> GraphQLUnion<S> for &T
where
    T: GraphQLUnion<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> GraphQLUnion<S> for Box<T>
where
    T: GraphQLUnion<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> GraphQLUnion<S> for Arc<T>
where
    T: GraphQLUnion<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
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

impl<S, T> IsOutputType<S> for &T
where
    T: IsOutputType<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> IsOutputType<S> for Box<T>
where
    T: IsOutputType<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> IsOutputType<S> for Arc<T>
where
    T: IsOutputType<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> IsOutputType<S> for Option<T>
where
    T: IsOutputType<S>,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> IsOutputType<S> for Vec<T>
where
    T: IsOutputType<S>,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> IsOutputType<S> for [T]
where
    T: IsOutputType<S>,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T, const N: usize> IsOutputType<S> for [T; N]
where
    T: IsOutputType<S>,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S> IsOutputType<S> for str where S: ScalarValue {}

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

impl<S, T> IsInputType<S> for &T
where
    T: IsInputType<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> IsInputType<S> for Box<T>
where
    T: IsInputType<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> IsInputType<S> for Arc<T>
where
    T: IsInputType<S> + ?Sized,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> IsInputType<S> for Option<T>
where
    T: IsInputType<S>,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> IsInputType<S> for Vec<T>
where
    T: IsInputType<S>,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T> IsInputType<S> for [T]
where
    T: IsInputType<S>,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S, T, const N: usize> IsInputType<S> for [T; N]
where
    T: IsInputType<S>,
    S: ScalarValue,
{
    #[inline]
    fn mark() {
        T::mark()
    }
}

impl<S> IsInputType<S> for str where S: ScalarValue {}
