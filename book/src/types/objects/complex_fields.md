Complex fields
==============

Using a plain [Rust struct][struct] for representing a [GraphQL object][0] is easy and trivial but does not cover every case. What if we need to express something non-trivial as a [GraphQL field][4], such as:
- Calling non-trivial logic while [executing][1] the [field][4] (like querying database, etc.).
- Accepting [field arguments][5].
- Defining a circular [GraphQL object][0], where one of its [fields][4] returns the type itself.
- Using some other (non-[struct]) [Rust] type to represent a [GraphQL object][0].

To support these more complicated use cases, we need a way to define a [GraphQL field][4] as a function. In [Juniper] this is achievable by placing the [`#[graphql_object]` attribute][3] on an [`impl` block][6], which turns its methods into [GraphQL fields][4]:
```rust
# extern crate juniper;
# use juniper::{GraphQLObject, graphql_object};
#
#[derive(GraphQLObject)]
struct Person {
    name: String,
    age: i32,
}

struct House {
    inhabitants: Vec<Person>,
}

// Defines the `House` GraphQL object.
#[graphql_object]
impl House {
    // Creates the field `inhabitantWithName(name: String!)`, 
    // returning a `null`able `Person`.
    fn inhabitant_with_name(&self, name: String) -> Option<&Person> {
        self.inhabitants.iter().find(|p| p.name == name)
    }
}
#
# fn main() {}
```
> **NOTE**: To access global data such as database connections or authentication information, a _context_ is used. To learn more about this, see the ["Context" chapter](context.md).


### Default arguments

Though [Rust] doesn't have the notion of default arguments, [GraphQL arguments][4] are able to have default values. These default values are used when a GraphQL operation doesn't specify the argument explicitly. In [Juniper], defining a default value for a [GraphQL argument][4] is enabled by the `#[graphql(default)]` attribute:
```rust
# extern crate juniper;
# use juniper::graphql_object;
#
struct Person;

#[graphql_object]
impl Person {
    fn field1(
        // Default value can be any valid Rust expression, including a function
        // call, etc.
        #[graphql(default = true)]
        arg1: bool,
        // If default expression is not specified, then the `Default::default()` 
        // value is used.
        #[graphql(default)]
        arg2: i32,
    ) -> String {
        format!("{arg1} {arg2}")
    }
}
#
# fn main() {}
```


### Renaming

Like with the [`#[derive(GraphQLObject)]` attribute on structs](index.md#renaming), [field][4] names are converted from [Rust]'s standard `snake_case` naming convention into [GraphQL]'s `camelCase` convention.

We can override the name by using the `#[graphql(name = "...")]` attribute:
```rust
# extern crate juniper;
# use juniper::graphql_object;
#
struct Person;

#[graphql_object]
#[graphql(name = "PersonObject")]
impl Person { // exposed as `PersonObject` in GraphQL schema
    #[graphql(name = "myCustomFieldName")]
    fn renamed_field( // exposed as `myCustomFieldName` in GraphQL schema
        #[graphql(name = "myArgument")]
        renamed_argument: bool, // exposed as `myArgument` in GraphQL schema
    ) -> bool {
        renamed_argument
    }
}
#
# fn main() {}
```

Or provide a different renaming policy for all the defined [fields][4]:
```rust
# extern crate juniper;
# use juniper::graphql_object;
#
struct Person;

#[graphql_object]
#[graphql(rename_all = "none")] // disables any renaming
impl Person {
    fn renamed_field( // exposed as `renamed_field` in GraphQL schema
        renamed_argument: bool, // exposed as `renamed_argument` in GraphQL schema
    ) -> bool {
        renamed_argument
    }
}
#
# fn main() {}
```
> **TIP**: Supported policies are: `SCREAMING_SNAKE_CASE`, `snake_case`, `camelCase` and `none` (disables any renaming).


### Documentation and deprecation

Similarly, [GraphQL fields][4] (and their [arguments][5]) may also be [documented][7] and [deprecated][9] via `#[graphql(description = "...")]` and `#[graphql(deprecated = "...")]`/[`#[deprecated]`][13] attributes:
```rust
# extern crate juniper;
# use juniper::graphql_object;
#
struct Person;

/// This doc comment is visible only in Rust API docs.
#[graphql_object]
#[graphql(description = "This description overwrites the one from doc comment.")]
impl Person {
    /// This doc comment is visible only in Rust API docs.
    #[graphql(description = "This description is visible only in GraphQL schema.")]
    fn empty() -> &'static str {
        ""
    }
    
    #[graphql(desc = "This description is visible only in GraphQL schema.")]
    //        ^^^^ shortcut for a `description` argument
    fn field(
        #[graphql(desc = "This description is visible only in GraphQL schema.")]
        arg: bool,
    ) -> bool {
        arg
    }

    /// This doc comment is visible in both Rust API docs and GraphQL schema 
    /// descriptions.
    #[graphql(deprecated = "Just because.")]
    fn deprecated_graphql(
        // Only `Null`able arguments or non-`Null` arguments with default values
        // can be deprecated.
        #[graphql(default, deprecated = "No need.")] arg: bool,
    ) -> bool {
        true
    }
    
    // Standard Rust's `#[deprecated]` attribute works too!
    #[deprecated(note = "Reason is optional, btw!")]
    fn deprecated_standard( // has no description in GraphQL schema
        // If no explicit deprecation reason is provided,
        // then the default "No longer supported" one is used.
        #[graphql(deprecated)] arg: Option<bool>,
    ) -> bool {
        false
    }
}
#
# fn main() {}
```
> **NOTE**: Only [GraphQL object][0]/[interface][11]/[input object][8] fields, [arguments][5] and [GraphQL enum][10] values can be [deprecated][9].


### Ignoring

By default, all methods of an [`impl` block][6] are exposed as [GraphQL fields][4]. If a method should not be exposed as a [GraphQL field][4], it should be defined in a separate [`impl` block][6] or marked with the `#[graphql(ignore)]` attribute:
```rust
# #![expect(dead_code, reason = "example")]
# extern crate juniper;
# use juniper::graphql_object;
#
struct Person {
    name: String,
    age: i32,
}

#[graphql_object]
impl Person {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn age(&self) -> i32 {
        self.age
    }

    #[graphql(ignore)]
    pub fn hidden_from_graphql(&self) {
        // whatever goes...
    }

    #[graphql(skip)]
    //        ^^^^ alternative naming, up to your preference
    pub fn also_hidden_from_graphql(&self) {
        // whatever goes...
    }
}

impl Person {
    pub fn not_even_considered_for_graphql(&self) {
        // whatever goes...
    }
}
#
# fn main() {}
```

> **TIP**: See more available features in the API docs of the [`#[graphql_object]`][3] attribute.




[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org
[struct]: https://doc.rust-lang.org/reference/items/structs.html

[0]: https://spec.graphql.org/October2021#sec-Objects
[1]: https://spec.graphql.org/October2021#sec-Execution
[2]: https://docs.rs/juniper/0.17.1/juniper/derive.GraphQLObject.html
[3]: https://docs.rs/juniper/0.17.1/juniper/attr.graphql_object.html
[4]: https://spec.graphql.org/October2021#sec-Language.Fields
[5]: https://spec.graphql.org/October2021#sec-Language.Arguments
[6]: https://doc.rust-lang.org/reference/items/implementations.html#inherent-implementations
[7]: https://spec.graphql.org/October2021#sec-Descriptions
[8]: https://spec.graphql.org/October2021#sec-Input-Objects
[9]: https://spec.graphql.org/October2021#sec--deprecated
[10]: https://spec.graphql.org/October2021#sec-Enums
[11]: https://spec.graphql.org/October2021#sec-Interfaces
[13]: https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-deprecated-attribute
