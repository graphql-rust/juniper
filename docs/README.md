# Juniper

> GraphQL server library for Rust

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