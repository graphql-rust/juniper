//! Marker traits for GraphQL types.

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
    #[allow(missing_docs)]
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        <Self as GraphQLType<S>>::name(info)
    }
}
