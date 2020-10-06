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

/// Maker trait for [GraphQL interfaces][1].
///
/// This trait extends the [`GraphQLType`] and is only used to mark an [interface][1]. During
/// compile this addition information is required to prevent unwanted structure compiling. If an
/// object requires this trait instead of the [`GraphQLType`], then it explicitly requires
/// [GraphQL interfaces][1]. Other types ([scalars][2], [enums][3], [objects][4], [input objects][5]
/// and [unions][6]) are not allowed.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
/// [2]: https://spec.graphql.org/June2018/#sec-Scalars
/// [3]: https://spec.graphql.org/June2018/#sec-Enums
/// [4]: https://spec.graphql.org/June2018/#sec-Objects
/// [5]: https://spec.graphql.org/June2018/#sec-Input-Objects
/// [6]: https://spec.graphql.org/June2018/#sec-Unions
pub trait GraphQLInterface<S: ScalarValue>: GraphQLType<S> {
    /// An arbitrary function without meaning.
    ///
    /// May contain compile timed check logic which ensures that types are used correctly according
    /// to the [GraphQL specification][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/
    fn mark() {}
}

/// Maker trait for [GraphQL unions][1].
///
/// This trait extends the [`GraphQLType`] and is only used to mark an [union][1]. During compile
/// this addition information is required to prevent unwanted structure compiling. If an object
/// requires this trait instead of the [`GraphQLType`], then it explicitly requires
/// [GraphQL unions][1]. Other types ([scalars][2], [enums][3], [objects][4], [input objects][5] and
/// [interfaces][6]) are not allowed.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
/// [2]: https://spec.graphql.org/June2018/#sec-Scalars
/// [3]: https://spec.graphql.org/June2018/#sec-Enums
/// [4]: https://spec.graphql.org/June2018/#sec-Objects
/// [5]: https://spec.graphql.org/June2018/#sec-Input-Objects
/// [6]: https://spec.graphql.org/June2018/#sec-Interfaces
pub trait GraphQLUnion<S: ScalarValue>: GraphQLType<S> {
    /// An arbitrary function without meaning.
    ///
    /// May contain compile timed check logic which ensures that types are used correctly according
    /// to the [GraphQL specification][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/
    fn mark() {}
}

/// Marker trait for types which can be used as output types.
///
/// The GraphQL specification differentiates between input and output
/// types. Each type which can be used as an output type should
/// implement this trait. The specification defines enum, scalar,
/// object, union, and interface as output types.
// TODO: Re-enable GraphQLType requirement in #682
pub trait IsOutputType<S: ScalarValue> /*: GraphQLType<S>*/ {
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
    T: IsInputType<S>,
    S: ScalarValue,
{
}

impl<S, T> IsOutputType<S> for Option<T>
where
    T: IsOutputType<S>,
    S: ScalarValue,
{
}

impl<S, T> IsOutputType<S> for Vec<T>
where
    T: IsOutputType<S>,
    S: ScalarValue,
{
}

impl<S, T> IsOutputType<S> for [T]
where
    T: IsOutputType<S>,
    S: ScalarValue,
{
}

impl<S, T> IsInputType<S> for Vec<T>
where
    T: IsInputType<S>,
    S: ScalarValue,
{
}

impl<S, T> IsInputType<S> for [T]
where
    T: IsInputType<S>,
    S: ScalarValue,
{
}

impl<'a, S, T> IsInputType<S> for &T
where
    T: IsInputType<S> + ?Sized,
    S: ScalarValue,
{
}
impl<'a, S, T> IsOutputType<S> for &T
where
    T: IsOutputType<S> + ?Sized,
    S: ScalarValue,
{
}

impl<S, T> IsInputType<S> for Box<T>
where
    T: IsInputType<S> + ?Sized,
    S: ScalarValue,
{
}
impl<S, T> IsOutputType<S> for Box<T>
where
    T: IsOutputType<S> + ?Sized,
    S: ScalarValue,
{
}

impl<'a, S> IsInputType<S> for str where S: ScalarValue {}
impl<'a, S> IsOutputType<S> for str where S: ScalarValue {}
