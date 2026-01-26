Input objects
=============

> [Fields][4] may accept [arguments][5] to configure their behavior. These inputs are often [scalars][12] or [enums][10], but they sometimes need to represent more complex values.
>
> A [GraphQL input object][0] defines a set of input fields; the input fields are either [scalars][12], [enums][10], or other [input objects][0]. This allows [arguments][5] to accept arbitrarily complex structs.

In [Juniper], defining a [GraphQL input object][0] is quite straightforward and similar to how [trivial GraphQL objects are defined](objects/index.md) - by using the [`#[derive(GraphQLInputObject)]` attribute][2] on a [Rust struct][struct]:
```rust
# #![expect(unused_variables, reason = "example")]
# extern crate juniper;
# use juniper::{GraphQLInputObject, GraphQLObject, graphql_object};
#
#[derive(GraphQLInputObject)]
struct Coordinate {
    latitude: f64,
    longitude: f64
}

struct Root;
# #[derive(GraphQLObject)] struct User { name: String }

#[graphql_object]
impl Root {
    fn users_at_location(coordinate: Coordinate, radius: f64) -> Vec<User> {
        // Send coordinate to database
        // ...
#       unimplemented!()
    }
}
#
# fn main() {}
```


### Renaming

Just as with [defining GraphQL objects](objects/index.md#renaming), by default [struct] fields are converted from [Rust]'s standard `snake_case` naming convention into [GraphQL]'s `camelCase` convention:
```rust
# extern crate juniper;
# use juniper::GraphQLInputObject;
#
#[derive(GraphQLInputObject)]
struct Person {
    first_name: String, // exposed as `firstName` in GraphQL schema
    last_name: String,  // exposed as `lastName` in GraphQL schema
}
#
# fn main() {}
```

We can override the name by using the `#[graphql(name = "...")]` attribute:
```rust
# extern crate juniper;
# use juniper::GraphQLInputObject;
#
#[derive(GraphQLInputObject)]
#[graphql(name = "WebPerson")] // now exposed as `WebPerson` in GraphQL schema
struct Person {
    name: String,
    age: i32,
    #[graphql(name = "websiteURL")]
    website_url: Option<String>, // now exposed as `websiteURL` in GraphQL schema
}
#
# fn main() {}
```

Or provide a different renaming policy for all the [struct] fields:
```rust
# extern crate juniper;
# use juniper::GraphQLInputObject;
#
#[derive(GraphQLInputObject)]
#[graphql(rename_all = "none")] // disables any renaming
struct Person {
    name: String,
    age: i32,
    website_url: Option<String>, // exposed as `website_url` in GraphQL schema
}
#
# fn main() {}
```
> **TIP**: Supported policies are: `SCREAMING_SNAKE_CASE`, `camelCase` and `none` (disables any renaming).


### Documentation

Similarly, [GraphQL descriptions][7] may be provided by either using [Rust doc comments][6] or with the `#[graphql(description = "...")]` attribute:
```rust
# extern crate juniper;
# use juniper::GraphQLInputObject;
#
/// This doc comment is visible only in Rust API docs.
#[derive(GraphQLInputObject)]
#[graphql(description = "This description is visible only in GraphQL schema.")]
struct Person {
    /// This doc comment is visible only in Rust API docs.
    #[graphql(desc = "This description is visible only in GraphQL schema.")]
    //        ^^^^ shortcut for a `description` argument
    name: String,

    /// This doc comment is visible in both Rust API docs and GraphQL schema 
    /// descriptions.
    age: i32,
}
#
# fn main() {}
```
> **NOTE**: As of [October 2021 GraphQL specification][spec], [GraphQL input object][0]'s fields **cannot be** [deprecated][9].


### Ignoring

By default, all [struct] fields are included into the generated [GraphQL input object][0] type. To prevent inclusion of a specific field annotate it with the `#[graphql(ignore)]` attribute:
> **WARNING**: Ignored fields must either implement `Default` or be annotated with the `#[graphql(default = <expression>)]` argument.
```rust
# extern crate juniper;
# use juniper::GraphQLInputObject;
#
enum System {
    Cartesian,
}

#[derive(GraphQLInputObject)]
struct Point2D {
    x: f64,
    y: f64,
    #[graphql(ignore, default = System::Cartesian)]
    //                ^^^^^^^^^^^^^^^^^^^^^^^^^^^
    // This attribute is required, as we need to be able to construct
    // a `Point2D` value from the `{ x: 0.0, y: 0.0 }` GraphQL input value,
    // received from client-side.
    system: System,
    // `Default::default()` value is used, if no 
    // `#[graphql(default = <expression>)]` is specified.
    #[graphql(skip)]
    //        ^^^^ alternative naming, up to your preference
    shift: f64, 
}
#
# fn main() {}
```

> **TIP**: See more available features in the API docs of the [`#[derive(GraphQLInputObject)]`][2] attribute.



[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org
[struct]: https://doc.rust-lang.org/reference/items/structs.html
[spec]: https://spec.graphql.org/October2021

[0]: https://spec.graphql.org/October2021#sec-Input-Objects
[2]: https://docs.rs/juniper/0.17.1/juniper/derive.GraphQLInputObject.html
[4]: https://spec.graphql.org/October2021#sec-Language.Fields
[5]: https://spec.graphql.org/October2021#sec-Language.Arguments
[6]: https://doc.rust-lang.org/reference/comments.html#doc-comments
[7]: https://spec.graphql.org/October2021#sec-Descriptions
[9]: https://spec.graphql.org/October2021#sec--deprecated
[10]: https://spec.graphql.org/October2021#sec-Enums
[12]: https://spec.graphql.org/October2021#sec-Scalars
