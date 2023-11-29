# Complex fields

If you've got a struct that can't be mapped directly to GraphQL, that contains
computed fields or circular structures, you have to use a more powerful tool:
the `#[graphql_object]` procedural macro. This macro lets you define GraphQL object
fields in a Rust `impl` block for a type. Note, that GraphQL fields are defined in 
this `impl` block by default. If you want to define normal methods on the struct,
you have to do so either in a separate "normal" `impl` block, or mark them with
`#[graphql(ignore)]` attribute to be omitted by the macro. Continuing with the
example from the last chapter, this is how you would define `Person` using the
macro:

```rust
# #![allow(dead_code)]
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
        // [...]
    }
}

impl Person {
    pub fn hidden_from_graphql2(&self) {
        // [...]
    }
}
#
# fn main() { }
```

While this is a bit more verbose, it lets you write any kind of function in the
field resolver. With this syntax, fields can also take arguments:

```rust
# extern crate juniper;
# use juniper::{graphql_object, GraphQLObject};
#
#[derive(GraphQLObject)]
struct Person {
    name: String,
    age: i32,
}

struct House {
    inhabitants: Vec<Person>,
}

#[graphql_object]
impl House {
    // Creates the field `inhabitantWithName(name)`, returning a nullable `Person`.
    fn inhabitant_with_name(&self, name: String) -> Option<&Person> {
        self.inhabitants.iter().find(|p| p.name == name)
    }
}
#
# fn main() {}
```

To access global data such as database connections or authentication
information, a _context_ is used. To learn more about this, see the next
chapter: [Using contexts](using_contexts.md).

## Description, renaming, and deprecation

Like with the derive attribute, field names will be converted from `snake_case`
to `camelCase`. If you need to override the conversion, you can simply rename
the field. Also, the type name can be changed with an alias:

```rust
# extern crate juniper;
# use juniper::graphql_object;
#
struct Person;

/// Doc comments are used as descriptions for GraphQL.
#[graphql_object(
    // With this attribute you can change the public GraphQL name of the type.
    name = "PersonObject",

    // You can also specify a description here, which will overwrite 
    // a doc comment description.
    description = "...",
)]
impl Person {
    /// A doc comment on the field will also be used for GraphQL.
    #[graphql(
        // Or provide a description here.
        description = "...",
    )]
    fn doc_comment(&self) -> &str {
        ""
    }

    // Fields can also be renamed if required.
    #[graphql(name = "myCustomFieldName")]
    fn renamed_field() -> bool {
        true
    }

    // Deprecations also work as you'd expect.
    // Both the standard Rust syntax and a custom attribute is accepted.
    #[deprecated(note = "...")]
    fn deprecated_standard() -> bool {
        false
    }

    #[graphql(deprecated = "...")]
    fn deprecated_graphql() -> bool {
        true
    }
}
#
# fn main() { }
```

Or provide a different renaming policy on a `impl` block for all its fields:
```rust
# extern crate juniper;
# use juniper::graphql_object;
struct Person;

#[graphql_object(rename_all = "none")] // disables any renaming
impl Person {
    // Now exposed as `renamed_field` in the schema
    fn renamed_field() -> bool {
        true
    }
}
#
# fn main() {}
```

## Customizing arguments

Method field arguments can also be customized.

They can have custom descriptions and default values.

```rust
# extern crate juniper;
# use juniper::graphql_object;
#
struct Person;

#[graphql_object]
impl Person {
    fn field1(
        &self,
        #[graphql(
            // Arguments can also be renamed if required.
            name = "arg",
            // Set a default value which will be injected if not present.
            // The default can be any valid Rust expression, including a function call, etc.
            default = true,
            // Set a description.
            description = "The first argument..."
        )]
        arg1: bool,
        // If default expression is not specified then `Default::default()` value is used.
        #[graphql(default)]
        arg2: i32,
    ) -> String {
        format!("{arg1} {arg2}")
    }
}
#
# fn main() { }
```

Provide a different renaming policy on a `impl` block also implies for arguments:
```rust
# extern crate juniper;
# use juniper::graphql_object;
struct Person;

#[graphql_object(rename_all = "none")] // disables any renaming
impl Person {
    // Now exposed as `my_arg` in the schema
    fn field(my_arg: bool) -> bool {
        my_arg
    }
}
#
# fn main() {}
```

## More features

These, and more features, are described more thoroughly in [the reference documentation](https://docs.rs/juniper/latest/juniper/attr.graphql_object.html).
