Enums
=====

> [GraphQL enum][0] types, like [scalar][1] types, also represent leaf values in a GraphQL type system. However [enum][0] types describe the set of possible values.
>
> [Enums][0] are not references for a numeric value, but are unique values in their own right. They may serialize as a string: the name of the represented value.

With [Juniper] a [GraphQL enum][0] may be defined by using the [`#[derive(GraphQLEnum)]`][2] attribute on a [Rust enum][3] as long as its variants do not have any fields:
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


### Renaming

By default, [enum][3] variants are converted from [Rust]'s standard `PascalCase` naming convention into [GraphQL]'s `SCREAMING_SNAKE_CASE` convention:
```rust
# extern crate juniper;
# use juniper::GraphQLEnum;
#
#[derive(GraphQLEnum)]
enum Episode {
    NewHope, // exposed as `NEW_HOPE` in GraphQL schema
    Empire,  // exposed as `EMPIRE` in GraphQL schema
    Jedi,    // exposed as `JEDI` in GraphQL schema
}
#
# fn main() {}
```

We can override the name by using the `#[graphql(name = "...")]` attribute:
```rust
# extern crate juniper;
# use juniper::GraphQLEnum;
#
#[derive(GraphQLEnum)]
#[graphql(name = "WrongEpisode")] // now exposed as `WrongEpisode` in GraphQL schema
enum Episode {
    #[graphql(name = "LAST_HOPE")]
    NewHope, // exposed as `LAST_HOPE` in GraphQL schema
    Empire,
    Jedi,
}
#
# fn main() {}
```

Or provide a different renaming policy for all the [enum][3] variants:
```rust
# extern crate juniper;
# use juniper::GraphQLEnum;
#
#[derive(GraphQLEnum)]
#[graphql(rename_all = "none")] // disables any renaming
enum Episode {
    NewHope, // exposed as `NewHope` in GraphQL schema
    Empire,  // exposed as `Empire` in GraphQL schema
    Jedi,    // exposed as `Jedi` in GraphQL schema
}
#
# fn main() {}
```
> **TIP**: Supported policies are: `SCREAMING_SNAKE_CASE`, `snake_case`, `camelCase` and `none` (disables any renaming).


### Documentation and deprecation

Just like when [defining GraphQL objects](objects/index.md#documentation), the [GraphQL enum][0] type and its values could be [documented][4] and [deprecated][9] via `#[graphql(description = "...")]` and `#[graphql(deprecated = "...")]`/[`#[deprecated]`][13] attributes:
```rust
# extern crate juniper;
# use juniper::GraphQLEnum;
#
/// This doc comment is visible only in Rust API docs.
#[derive(GraphQLEnum)]
#[graphql(description = "An episode of Star Wars")]
enum StarWarsEpisode {
    /// This doc comment is visible only in Rust API docs.
    #[graphql(description = "This description is visible only in GraphQL schema.")]
    NewHope,

    /// This doc comment is visible only in Rust API docs.
    #[graphql(desc = "Arguably the best one in the trilogy.")]
    //        ^^^^ shortcut for a `description` argument
    Empire,

    /// This doc comment is visible in both Rust API docs and GraphQL schema 
    /// descriptions.
    Jedi,
    
    #[deprecated(note = "Only visible in Rust.")]
    #[graphql(deprecated = "We don't really talk about this one.")]
    //        ^^^^^^^^^^ takes precedence over Rust's `#[deprecated]` attribute
    ThePhantomMenace, // has no description in GraphQL schema
}
#
# fn main() {}
```
> **NOTE**: Only [GraphQL object][6]/[interface][7]/[input object][8] fields, [arguments][5] and [GraphQL enum][0] values can be [deprecated][9].


### Ignoring

By default, all [enum][3] variants are included in the generated [GraphQL enum][0] type as values. To prevent including a specific variant, annotate it with the `#[graphql(ignore)]` attribute:
```rust
# #![expect(dead_code, reason = "example")]
# extern crate juniper;
# use juniper::GraphQLEnum;
#
#[derive(GraphQLEnum)]
enum Episode<T> {
    NewHope,
    Empire,
    Jedi,
    #[graphql(ignore)]
    Legends(T),   // cannot be queried from GraphQL
    #[graphql(skip)]
    //        ^^^^ alternative naming, up to your preference
    CloneWars(T), // cannot be queried from GraphQL
}
#
# fn main() {}
```

> **TIP**: See more available features in the API docs of the [`#[derive(GraphQLEnum)]`][2] attribute.




[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org

[0]: https://spec.graphql.org/October2021#sec-Enums
[1]: https://spec.graphql.org/October2021#sec-Scalars
[2]: https://docs.rs/juniper/0.17.1/juniper/derive.GraphQLEnum.html
[3]: https://doc.rust-lang.org/reference/items/enumerations.html
[4]: https://spec.graphql.org/October2021#sec-Descriptions
[5]: https://spec.graphql.org/October2021#sec-Language.Arguments
[6]: https://spec.graphql.org/October2021#sec-Objects
[7]: https://spec.graphql.org/October2021#sec-Interfaces
[8]: https://spec.graphql.org/October2021#sec-Input-Objects
[9]: https://spec.graphql.org/October2021#sec--deprecated
[13]: https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-deprecated-attribute
