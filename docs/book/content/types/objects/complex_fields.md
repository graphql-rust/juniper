# Complex fields

If you've got a struct that can't be mapped directly to GraphQL, that contains
computed fields or circular structures, you have to use a more powerful tool:
the `object` procedural macro. This macro lets you define GraphQL object
fields in a Rust `impl` block for a type. Continuing with the
example from the last chapter, this is how you would define `Person` using the
macro:

```rust

struct Person {
    name: String,
    age: i32,
}

#[juniper::object]
impl Person {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn age(&self) -> i32 {
        self.age
    }
}

# fn main() { }
```

While this is a bit more verbose, it lets you write any kind of function in the
field resolver. With this syntax, fields can also take arguments:


```rust
#[derive(juniper::GraphQLObject)]
struct Person {
    name: String,
    age: i32,
}

struct House {
    inhabitants: Vec<Person>,
}

#[juniper::object]
impl House {
    // Creates the field inhabitantWithName(name), returning a nullable person
    fn inhabitant_with_name(&self, name: String) -> Option<&Person> {
        self.inhabitants.iter().find(|p| p.name == name)
    }
}

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
struct Person {
    name: String,
    website_url: String,
}

#[juniper::object(
    // With this attribtue you can change the public GraphQL name of the type.
    name = "PersonObject",
)]
impl Person {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn websiteURL(&self) -> &str {
        self.website_url.as_str()
    }
}

# fn main() { }
```

## More features

GraphQL fields expose more features than Rust's standard method syntax gives us:

* Per-field description and deprecation messages
* Per-argument default values
* Per-argument descriptions

These, and more features, are described more thorougly in [the reference
documentation](https://docs.rs/juniper/latest/juniper/macro.object.html).
