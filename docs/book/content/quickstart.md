# Quickstart

This page will give you a short introduction to the concepts in Juniper.

## Installation

!FILENAME Cargo.toml

```toml
[dependencies]
juniper = "0.11"
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

```rust
use juniper::{FieldResult};

# struct DatabasePool;
# impl DatabasePool {
#     fn get_connection(&self) -> FieldResult<DatabasePool> { Ok(DatabasePool) }
#     fn find_human(&self, _id: &str) -> FieldResult<Human> { Err("")? }
#     fn insert_human(&self, _human: &NewHuman) -> FieldResult<Human> { Err("")? }
# }

#[derive(juniper::GraphQLEnum)]
enum Episode {
    NewHope,
    Empire,
    Jedi,
}

#[derive(juniper::GraphQLObject)]
#[graphql(description="A humanoid creature in the Star Wars universe")]
struct Human {
    id: String,
    name: String,
    appears_in: Vec<Episode>,
    home_planet: String,
}

// There is also a custom derive for mapping GraphQL input objects.

#[derive(juniper::GraphQLInputObject)]
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

// To make our context usable by Juniper, we have to implement a marker trait.
impl juniper::Context for Context {}

struct Query;

juniper::graphql_object!(Query: Context |&self| {

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
        let human = connection.find_human(&id)?;
        // Return the result.
        Ok(human)
    }
});

struct Mutation;

juniper::graphql_object!(Mutation: Context |&self| {

    field createHuman(&executor, new_human: NewHuman) -> FieldResult<Human> {
        let db = executor.context().pool.get_connection()?;
        let human: Human = db.insert_human(&new_human)?;
        Ok(human)
    }
});

// A root schema consists of a query and a mutation.
// Request queries can be executed against a RootNode.
type Schema = juniper::RootNode<'static, Query, Mutation>;

# fn main() {
#   let _ = Schema::new(Query, Mutation{});
# }
```

We now have a very simple but functional schema for a GraphQL server!

To actually serve the schema, see the guides for our various [server integrations](./servers/index.md).

You can also invoke the executor directly to get a result for a query:

## Executor

You can invoke `juniper::execute` directly to run a GraphQL query:

```rust
# // Only needed due to 2018 edition because the macro is not accessible.
# #[macro_use]
# extern crate juniper;
use juniper::{FieldResult, Variables, EmptyMutation};

#[derive(juniper::GraphQLEnum, Clone, Copy)]
enum Episode {
    NewHope,
    Empire,
    Jedi,
}

struct Query;

juniper::graphql_object!(Query: Ctx |&self| {
    field favoriteEpisode(&executor) -> FieldResult<Episode> {
        // Use the special &executor argument to fetch our fav episode.
        Ok(executor.context().0)
    }
});

// Arbitrary context data.
struct Ctx(Episode);

// A root schema consists of a query and a mutation.
// Request queries can be executed against a RootNode.
type Schema = juniper::RootNode<'static, Query, EmptyMutation<Ctx>>;

fn main() {
    // Create a context object.
    let ctx = Ctx(Episode::NewHope);

    // Run the executor.
    let (res, _errors) = juniper::execute(
        "query { favoriteEpisode }",
        None,
        &Schema::new(Query, EmptyMutation::new()),
        &Variables::new(),
        &ctx,
    ).unwrap();

    // Ensure the value matches.
    assert_eq!(
        res,
        graphql_value!({
            "favoriteEpisode": "NEW_HOPE",
        })
    );
}
```

[hyper]: servers/hyper.md
[warp]: servers/warp.md
[rocket]: servers/rocket.md
[iron]: servers/iron.md
[tutorial]: ./tutorial.html
[jp_obj_macro]: https://docs.rs/juniper/latest/juniper/macro.graphql_object.html
