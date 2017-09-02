# Juniper

Juniper is a [GraphQL] server library for Rust. Build type-safe and fast API
servers with minimal boilerplate and configuration.

The library itself does not contain a web server - we currently have
integrations with [Rocket] and [Iron] depending on your needs.

## Installation

!FILENAME Cargo.toml
```toml
[dependencies]
juniper = { git = "https://github.com/graphql-rust/juniper" }
juniper_codegen = { git = "https://github.com/graphql-rust/juniper" }
```

## Schema example

Exposing simple enums and structs as GraphQL is just a matter of adding a custom
derive attribute to them. Juniper includes support for basic Rust types that
naturally map to GraphQL features, such as `Option<T>`, `Vec<T>`, `Box<T>`,
`String`, `f64`, and `i32`, references, and slices.

```rust
extern crate juniper;
#[macro_use] extern crate juniper_codegen;

#[derive(GraphQLEnum)]
enum Episode {
    NewHope,
    Empire,
    Jedi,
}

#[derive(GraphQLObject)]
#[graphql(description="A humanoid creature in the Star Wars universe")]
struct Human {
    id: String,
    name: String,
    appears_in: Vec<Episode>,
    home_planet: String,
}

# fn main() { }
```

[GraphQL]: https://graphql.org
[Iron]: http://ironframework.org
[Rocket]: https://rocket.rs