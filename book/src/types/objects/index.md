Objects
=======

> [GraphQL objects][0] represent a list of named fields, each of which yield a value of a specific type.

When declaring a [GraphQL schema][schema], most of the time we deal with [GraphQL objects][0], because they are the only place where we actually define the behavior once [schema] gets [executed][1].

There are two ways to define a [GraphQL object][0] in [Juniper]:
1. The easiest way, suitable for trivial cases, is to use the [`#[derive(GraphQLObject)]` attribute][2] on a [struct], as described below.
2. The other way, using the [`#[graphql_object]` attribute][3], is described in the ["Complex fields" chapter](complex_fields.md).




## Trivial

While any type in [Rust] can be exposed as a [GraphQL object][0], the most common one is a [struct]:
```rust
# extern crate juniper;
# use juniper::GraphQLObject;
#
#[derive(GraphQLObject)]
struct Person {
    name: String,
    age: i32,
}
#
# fn main() {}
```
This creates a [GraphQL object][0] type called `Person`, with two fields: `name` of type `String!`, and `age` of type `Int!`. Because of [Rust]'s type system, everything is exported as [non-`null`][4] by default.

> **TIP**: If a `null`able field is required, the most obvious way is to use `Option`. Or [`Nullable`] for distinguishing between [explicit and implicit `null`s][14].


### Documentation

We should take advantage of the fact that [GraphQL] is [self-documenting][15] and add descriptions to the defined [GraphQL object][0] type and its fields. [Juniper] will automatically use associated [Rust doc comments][6] as [GraphQL descriptions][7]:
```rust
# extern crate juniper;
# use juniper::GraphQLObject;
#
/// Information about a person.
#[derive(GraphQLObject)]
struct Person {
    /// The person's full name, including both first and last names.
    name: String,

    /// The person's age in years, rounded down.
    age: i32,
}
#
# fn main() {}
```

If using [Rust doc comments][6] is not desired (for example, when we want to keep [Rust] API docs and GraphQL schema descriptions different), the `#[graphql(description = "...")]` attribute can be used instead, which takes precedence over [Rust doc comments][6]:
```rust
# extern crate juniper;
# use juniper::GraphQLObject;
#
/// This doc comment is visible only in Rust API docs.
#[derive(GraphQLObject)]
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


### Renaming

By default, [struct] fields are converted from [Rust]'s standard `snake_case` naming convention into [GraphQL]'s `camelCase` convention:
```rust
# extern crate juniper;
# use juniper::GraphQLObject;
#
#[derive(GraphQLObject)]
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
# use juniper::GraphQLObject;
#
#[derive(GraphQLObject)]
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
# use juniper::GraphQLObject;
#
#[derive(GraphQLObject)]
#[graphql(rename_all = "none")] // disables any renaming
struct Person {
    name: String,
    age: i32,
    website_url: Option<String>, // exposed as `website_url` in GraphQL schema
}
#
# fn main() {}
```
> **TIP**: Supported policies are: `SCREAMING_SNAKE_CASE`, `snake_case`, `camelCase` and `none` (disables any renaming).


### Deprecation

To [deprecate][9] a [GraphQL object][0] field, either the `#[graphql(deprecated = "...")]` attribute, or [Rust's `#[deprecated]` attribute][13], should be used:
```rust
# extern crate juniper;
# use juniper::GraphQLObject;
#
#[derive(GraphQLObject)]
struct Person {
    name: String,
    age: i32,
    #[graphql(deprecated = "Please use the `name` field instead.")]
    first_name: String,
    #[deprecated(note = "Please use the `name` field instead.")]
    last_name: String,
}
#
# fn main() {}
```
> **NOTE**: Only [GraphQL object][0]/[interface][11]/[input object][8] fields, [arguments][5] and [GraphQL enum][10] values can be [deprecated][9].


### Ignoring

By default, all [struct] fields are included into the generated [GraphQL object][0] type. To prevent inclusion of a specific field annotate it with the `#[graphql(ignore)]` attribute:
```rust
# #![expect(dead_code, reason = "example")]
# extern crate juniper;
# use juniper::GraphQLObject;
#
#[derive(GraphQLObject)]
struct Person {
    name: String,
    age: i32,
    #[graphql(ignore)]
    password_hash: String, // cannot be queried from GraphQL
    #[graphql(skip)]
    //        ^^^^ alternative naming, up to your preference
    is_banned: bool,       // cannot be queried from GraphQL
}
#
# fn main() {}
```

> **TIP**: See more available features in the API docs of the [`#[derive(GraphQLObject)]`][2] attribute.




## Relationships

[GraphQL object][0] fields can be of any [GraphQL] type, except [input objects][8].

Let's see what it means to build relationships between [objects][0]:
```rust
# extern crate juniper;
# use juniper::GraphQLObject;
#
#[derive(GraphQLObject)]
struct Person {
    name: String,
    age: i32,
}

#[derive(GraphQLObject)]
struct House {
    address: Option<String>,  // converted into `String` (`null`able)
    inhabitants: Vec<Person>, // converted into `[Person!]!`
}
#
# fn main() {}
```

Because `Person` is a valid [GraphQL] type, we can have a `Vec<Person>` in a [struct], and it'll be automatically converted into a [list][12] of [non-`null`able][4] `Person` [objects][0].




[`Nullable`]: https://docs.rs/juniper/0.17.0/juniper/enum.Nullable.html
[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org
[schema]: https://graphql.org/learn/schema
[struct]: https://doc.rust-lang.org/reference/items/structs.html

[0]: https://spec.graphql.org/October2021#sec-Objects
[1]: https://spec.graphql.org/October2021#sec-Execution
[2]: https://docs.rs/juniper/0.17.0/juniper/derive.GraphQLObject.html
[3]: https://docs.rs/juniper/0.17.0/juniper/attr.graphql_object.html
[4]: https://spec.graphql.org/October2021#sec-Non-Null
[5]: https://spec.graphql.org/October2021#sec-Language.Arguments
[6]: https://doc.rust-lang.org/reference/comments.html#doc-comments
[7]: https://spec.graphql.org/October2021#sec-Descriptions
[8]: https://spec.graphql.org/October2021#sec-Input-Objects
[9]: https://spec.graphql.org/October2021#sec--deprecated
[10]: https://spec.graphql.org/October2021#sec-Enums
[11]: https://spec.graphql.org/October2021#sec-Interfaces
[12]: https://spec.graphql.org/October2021#sec-List
[13]: https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-deprecated-attribute
[14]: https://spec.graphql.org/October2021#sel-EAFdRDHAAEJDAoBxzT
[15]: https://spec.graphql.org/October2021#sec-Introspection
