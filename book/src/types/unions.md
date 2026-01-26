Unions
======

> [GraphQL unions][0] represent an object that could be one of a list of [GraphQL object][10] types, but provides for no guaranteed fields between those types. They also differ from [interfaces][12] in that [object][10] types declare what [interfaces][12] they implement, but are not aware of what [unions][0] contain them.

From the server's point of view, [GraphQL unions][0] are somewhat similar to [interfaces][12]: the main difference is that they don't contain fields on their own, and so, we only need to represent a value, _dispatchable_ into concrete [objects][10].

Obviously, the most straightforward approach to express [GraphQL unions][0] in [Rust] is to use [enums][22]. In [Juniper] this may be done by using [`#[derive(GraphQLUnion)]`][2] attribute on them:
```rust
# extern crate derive_more;
# extern crate juniper;
# use derive_more::From;
# use juniper::{GraphQLObject, GraphQLUnion};
# 
#[derive(GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

#[derive(From, GraphQLUnion)]
//       ^^^^ only for convenience, and may be omitted
enum Character {
    Human(Human),
    Droid(Droid),
}
#
# fn main() {}
```


### Renaming

Just as with [renaming GraphQL objects](objects/index.md#renaming), we can override the default [union][0] name by using the `#[graphql(name = "...")]` attribute:
```rust
# extern crate juniper;
# use juniper::{GraphQLObject, GraphQLUnion};
# 
# #[derive(GraphQLObject)]
# struct Human {
#     id: String,
#     home_planet: String,
# }
#
# #[derive(GraphQLObject)]
# struct Droid {
#     id: String,
#     primary_function: String,
# }
#
#[derive(GraphQLUnion)]
#[graphql(name = "CharacterUnion")]
enum Character { // exposed as `CharacterUnion` in GraphQL schema
    Human(Human),
    Droid(Droid),
}
#
# fn main() {}
```
> **NOTE**: Unlike [Rust enum variants][22], [GraphQL union members][0] don't have any special names aside from the ones provided by [objects][10] themselves, and so, obviously, **cannot be renamed**.


### Documentation

Similarly to [documenting GraphQL objects](objects/index.md#documentation), we can [document][7] a [GraphQL union][0] via `#[graphql(description = "...")]` attribute or [Rust doc comments][6]:
```rust
# extern crate juniper;
# use juniper::{GraphQLObject, GraphQLUnion};
# 
# #[derive(GraphQLObject)]
# struct Human {
#     id: String,
#     home_planet: String,
# }
#
# #[derive(GraphQLObject)]
# struct Droid {
#     id: String,
#     primary_function: String,
# }
#
/// This doc comment is visible in both Rust API docs and GraphQL schema 
/// descriptions.
#[derive(GraphQLUnion)]
enum Character {
    /// This doc comment is visible only in Rust API docs.
    Human(Human),
    /// This doc comment is visible only in Rust API docs.
    Droid(Droid),
}

/// This doc comment is visible only in Rust API docs.
#[derive(GraphQLUnion)]
#[graphql(description = "This description overwrites the one from doc comment.")]
//        ^^^^^^^^^^^ or `desc` shortcut, up to your preference
enum Person {
    /// This doc comment is visible only in Rust API docs.
    Human(Human),
}
#
# fn main() {}
```
> **NOTE**: Unlike [Rust enum variants][22], [GraphQL union members][0] don't have any special constructors aside from the provided [objects][10] directly, and so, **cannot be [documented][7]**, but rather reuse [object descriptions][7] "as is".


### Ignoring

In some rare situations we may want to omit exposing an [enum][22] variant in a [GraphQL schema][1]. [Similarly to GraphQL enums](enums.md#ignoring), we can just annotate the variant with the `#[graphql(ignore)]` attribute.

As an example, let's consider the situation where we need to bind some type parameter `T` for doing interesting type-level stuff in our resolvers. To achieve this we need to have `PhantomData<T>`, but we don't want it exposed in the GraphQL schema.

```rust
# extern crate derive_more;
# extern crate juniper;
# use std::marker::PhantomData;
# use derive_more::From;
# use juniper::{GraphQLObject, GraphQLUnion};
#
#[derive(GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

#[derive(From, GraphQLUnion)]
enum Character<S> {
    Human(Human),
    Droid(Droid),
    #[from(ignore)]
    #[graphql(ignore)]  
    //        ^^^^^^ or `skip`, up to your preference
    _State(PhantomData<S>),
}
#
# fn main() {}
```
> **WARNING**: It's the _library user's responsibility_ to ensure that ignored [enum][22] variant is **never** returned from resolvers, otherwise resolving the [GraphQL] query will **panic in runtime**.

> **TIP**: See more available features in the API docs of the [`#[derive(GraphQLUnion)]`][2] attribute.




[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org

[0]: https://spec.graphql.org/October2021#sec-Unions
[1]: https://graphql.org/learn/schema
[2]: https://docs.rs/juniper/0.17.1/juniper/derive.GraphQLUnion.html
[6]: https://doc.rust-lang.org/reference/comments.html#doc-comments
[7]: https://spec.graphql.org/October2021#sec-Descriptions
[10]: https://spec.graphql.org/October2021#sec-Objects
[11]: https://spec.graphql.org/October2021#sec-Enums
[12]: https://spec.graphql.org/October2021#sec-Interfaces
[22]: https://doc.rust-lang.org/reference/items/enumerations.html#enumerations
