//! Compile-time reflection of Rust types into GraphQL types.

use crate::behavior;

/// Alias for a [GraphQL type][0]'s name in a GraphQL schema.
///
/// See [`BaseType`] for more info.
///
/// [0]: https://spec.graphql.org/October2021#sec-Types
pub type Type = &'static str;

/// Alias for a slice of [`Type`]s.
///
/// See [`BaseSubTypes`] for more info.
pub type Types = &'static [Type];

/// Basic reflection of a [GraphQL type][0].
///
/// This trait is transparent to [`Option`], [`Vec`] and other containers, so to
/// fully represent a [GraphQL object][1] we additionally use [`WrappedType`].
///
/// [0]: https://spec.graphql.org/October2021#sec-Types
pub trait BaseType<Behavior: ?Sized = behavior::Standard> {
    /// [`Type`] of this [GraphQL type][0].
    ///
    /// Different Rust types may have the same [`NAME`]. For example, [`String`]
    /// and [`&str`](prim@str) share the `String!` GraphQL [`Type`].
    ///
    /// [`NAME`]: Self::NAME
    /// [0]: https://spec.graphql.org/October2021#sec-Types
    const NAME: Type;
}

/// Reflection of [sub-types][2] of a [GraphQL type][0].
///
/// This trait is transparent to [`Option`], [`Vec`] and other containers.
///
/// [0]: https://spec.graphql.org/October2021#sec-Types
/// [2]: https://spec.graphql.org/October2021#sel-JAHZhCHCDEJDAAAEEFDBtzC
pub trait BaseSubTypes<Behavior: ?Sized = behavior::Standard> {
    /// Sub-[`Types`] of this [GraphQL type][0].
    ///
    /// Contains [at least][2] the [`BaseType::NAME`] of this [GraphQL type][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Types
    /// [2]: https://spec.graphql.org/October2021#sel-JAHZhCHCDEJDAAAEEFDBtzC
    const NAMES: Types;
}

/// Alias for a value of a [`WrappedType`] (composed
/// [GraphQL wrapping type][0]).
///
/// [0]: https://spec.graphql.org/October2021#sec-Wrapping-Types
pub type WrappedValue = u128;
