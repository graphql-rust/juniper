//! Helper traits and definitions for macros.

pub mod subscription;

use crate::{DynGraphQLValue, DynGraphQLValueAsync};

/// Conversion of a [`GraphQLValue`] to its [trait object][1].
///
/// [`GraphQLValue`]: crate::GraphQLValue
/// [1]: https://doc.rust-lang.org/reference/types/trait-object.html
pub trait AsDynGraphQLValue {
    /// Context type of this [`GraphQLValue`].
    ///
    /// [`GraphQLValue`]: crate::GraphQLValue
    type Context;

    /// Schema information type of this [`GraphQLValue`].
    ///
    /// [`GraphQLValue`]: crate::GraphQLValue
    type TypeInfo;

    /// Converts this value to a [`DynGraphQLValue`] [trait object][1].
    ///
    /// [1]: https://doc.rust-lang.org/reference/types/trait-object.html
    fn as_dyn_graphql_value(&self) -> &DynGraphQLValue<Self::Context, Self::TypeInfo>;

    /// Converts this value to a [`DynGraphQLValueAsync`] [trait object][1].
    ///
    /// [1]: https://doc.rust-lang.org/reference/types/trait-object.html
    fn as_dyn_graphql_value_async(&self) -> &DynGraphQLValueAsync<Self::Context, Self::TypeInfo>;
}

crate::sa::assert_obj_safe!(AsDynGraphQLValue<Context = (), TypeInfo = ()>);
