# Quickstart

This page will give you a short introduction to the concepts in Juniper.

Once you are done here, head over to the [Tutorial][tutorial] to learn how to 
use Juniper by creating a full setup step by step, or consult the other chapters
for more detailed information.


## Installation

!FILENAME Cargo.toml
```toml
[dependencies]
juniper = "0.9.0"
```

## Schema example

Exposing simple enums and structs as GraphQL is just a matter of adding a custom
derive attribute to them. Juniper includes support for basic Rust types that
naturally map to GraphQL features, such as `Option<T>`, `Vec<T>`, `Box<T>`,
`String`, `f64`, and `i32`, references, and slices.

For more advanced mappings, Juniper provides multiple macros to map your Rust
types to a GraphQL schema. The most important one is the 
[graphql_object!][jp_obj_macro] macro that is used for declaring an object with
resolvers, which you will use for the `Query` and `Mutation` roots.

!FILENAME main.rs
```rust

#[macro_use] extern crate juniper;

use juniper::{FieldResult};
# struct DatabasePool;
# impl DatabasePool {
#   fn get_connection(&self) -> FieldResult<DatabasePool> { DatabasePool }
#   fn find_human(&self, id: &str) -> FieldResult<Human> { Err("")? }
#   fn insert_human(&self, human: &Human) -> FieldResult<()> { Ok(()) }
# }

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

// There is also a custom derive for mapping GraphQL input objects. 

#[derive(GraphQLInputObject)]
#[graphql(description="A humanoid creature in the Star Wars universe")]
struct NewHuman {
    name: String,
    appears_in: Vec<Episode>,
    home_planet: String,
}

// Now, we create our root Query and Mutation types with resolvers by using the 
// graphql_object! macro.
// Objects can have contexts that allow accessing shared state like a database
// pool.

struct Context {
    // Use your real database pool here.
    pool: DatabasePool,
}

struct Query;

graphql_object!(Query: Context |&self| {
    
    field apiVersion() -> &str {
        "1.0" 
    } 
    
    // Arguments to resolvers can either be simple types or input objects.
    // The executor is a special (optional) argument that allows accessing the context.
    field human(&executor, id: String) -> FieldResult<Human> {
        // Get the context from the executor.
        let context = executor.context();
        // Get a db connection.
        let connection = context.pool.get_connection()?;
        // Execute a db query.
        // Note the use of `?` to propagate errors.
        let human = context.db.find_human(&id)?;
        // Return the result.
        Ok(human)
    }
});

struct Mutation;

graphql_object!(Mutation: Context |&self| {
    
    field createHuman(&executor, new_human: NewHuman) -> FieldResult<Human> {
        let db = executor.context().get_connection()?;
        let human: Human = context.db.insert_human(&new_human)?;
        Ok(human)
    }
});

// A root schema consists of a query and a mutation.
// Request queries can be executed against a RootNode.
type Schema = juniper::RootNode<Query, Mutation>;

# fn main() { }
```

We now have a very simple but functional schema for a GraphQL server!

To actually serve the schema, see the guides for our [Rocket][rocket_guide] or 
[Iron][iron_guide] integrations.

[tutorial]: ./tutorial.html
[jp_obj_macro]: https://docs.rs/juniper/0.9.0/juniper/macro.graphql_object.html
[rocket_guide]: ./servers/rocket.html
[iron_guide]: ./servers/iron.html
