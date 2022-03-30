Enums
=====

Enums in GraphQL are string constants grouped together to represent a set of
possible values. Simple Rust enums can be converted to GraphQL enums by using a
custom derive attribute:

```rust
# extern crate juniper;
# use juniper::GraphQLEnum;
#
#[derive(GraphQLEnum)]
enum Episode {
    NewHope,
    Empire,
    Jedi,
}
#
# fn main() {}
```

## Custom name, description and deprecation

The name of [GraphQL enum][1] or its variants may be overridden with a `name`
attribute's argument. By default, a type name is used or `SCREAMING_SNAKE_CASE`
variant name.

The description of [GraphQL enum][1] or its variants may be specified either
with a `description`/`desc` attribute's argument, or with a regular Rust doc
comment.

A variant of [GraphQL enum][1] may be deprecated by specifying a `deprecated`
attribute's argument, or with regular Rust `#[deprecated]` attribute.

```rust
# extern crate juniper;
# use juniper::GraphQLEnum;
#
#[derive(GraphQLEnum)]
#[graphql(
    // Rename the type for GraphQL by specifying the name here.
    name = "AvailableEpisodes",
    // You may also specify a description here.
    // If present, doc comments will be ignored.
    desc = "Possible episodes.",
)]
enum Episode {
    /// Doc comment, also acting as description.
     #[deprecated(note = "Don't use it")]
    NewHope,
 
    #[graphql(name = "Jedi", desc = "Arguably the best one in the trilogy")]
    #[graphql(deprecated = "Don't use it")]
    Jedi,
 
    Empire,
}
#
# fn main() {}
```

## Renaming policy

By default, all [GraphQL enum][1] variants are renamed via
`SCREAMING_SNAKE_CASE` policy (so `NewHope` becomes `NEW_HOPE` variant in
GraphQL schema, and so on). This complies with default GraphQL naming 
conventions [demonstrated in spec][1].

However, if you need for some reason apply another naming convention, it's
possible to do by using `rename_all` attribute's argument. At the moment it
supports the following policies only: `SCREAMING_SNAKE_CASE`, `camelCase`,
`none` (disables any renaming).

```rust
# extern crate juniper;
# use juniper::GraphQLEnum;
#
#[derive(GraphQLEnum)]
#[graphql(rename_all = "none")] // disables renaming
enum Episode {
    NewHope,
    Empire,
    Jedi,
}
#
# fn main() {}
```

## Ignoring struct fields

To omit exposing a struct field in the GraphQL schema, use an `ignore` 
attribute's argument directly on that field. Only ignored variants can contain
fields.

```rust
# extern crate juniper;
# use juniper::GraphQLEnum;
#
#[derive(GraphQLEnum)]
enum Episode<T> {
    NewHope,
    Empire,
    Jedi,
    #[graphql(ignore)]
    Legends(T),
}
#
# fn main() {}
```

## Custom `ScalarValue`

By default, `#[derive(GraphQLEnum)]` macro generates code, which is generic over
a [`ScalarValue`] type. This can be changed with `scalar` attribute.

```rust
# extern crate juniper;
# use juniper::{DefaultScalarValue, GraphQLEnum};
#
#[derive(GraphQLEnum)]
#[graphql(scalar = DefaultScalarValue)]
enum Episode {
    NewHope,
    Empire,
    Jedi,
}
#
# fn main() {}
```




[1]: https://spec.graphql.org/October2021/#sec-Enums
