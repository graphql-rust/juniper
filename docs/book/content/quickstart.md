# Quickstart

This page will give you a short introduction to the concepts in Juniper.

Juniper follows a [code-first approach][schema_approach] to defining GraphQL schemas. If you would like to use a [schema-first approach][schema_approach] instead, consider [juniper-from-schema][] for generating code from a schema file.

## Installation

!FILENAME Cargo.toml

```toml
[dependencies]
juniper = { git = "https://github.com/graphql-rust/juniper" }
```

## Schema example

Exposing simple enums and structs as GraphQL is just a matter of adding a custom
derive attribute to them. Juniper includes support for basic Rust types that
naturally map to GraphQL features, such as `Option<T>`, `Vec<T>`, `Box<T>`,
`String`, `f64`, and `i32`, references, and slices.

For more advanced mappings, Juniper provides multiple macros to map your Rust
types to a GraphQL schema. The most important one is the
[graphql_object][graphql_object] procedural macro that is used for declaring an object with
resolvers, which you will use for the `Query` and `Mutation` roots.

```rust
# #![allow(unused_variables)]
# extern crate juniper;
# use std::fmt::Display;
use juniper::{
    graphql_object, EmptySubscription, FieldResult, GraphQLEnum, 
    GraphQLInputObject, GraphQLObject, ScalarValue,
};
#
# struct DatabasePool;
# impl DatabasePool {
#     fn get_connection(&self) -> FieldResult<DatabasePool> { Ok(DatabasePool) }
#     fn find_human(&self, _id: &str) -> FieldResult<Human> { Err("")? }
#     fn insert_human(&self, _human: &NewHuman) -> FieldResult<Human> { Err("")? }
# }

#[derive(GraphQLEnum)]
enum Episode {
    NewHope,
    Empire,
    Jedi,
}

#[derive(GraphQLObject)]
#[graphql(description = "A humanoid creature in the Star Wars universe")]
struct Human {
    id: String,
    name: String,
    appears_in: Vec<Episode>,
    home_planet: String,
}

// There is also a custom derive for mapping GraphQL input objects.

#[derive(GraphQLInputObject)]
#[graphql(description = "A humanoid creature in the Star Wars universe")]
struct NewHuman {
    name: String,
    appears_in: Vec<Episode>,
    home_planet: String,
}

// Now, we create our root Query and Mutation types with resolvers by using the
// object macro.
// Objects can have contexts that allow accessing shared state like a database
// pool.

struct Context {
    // Use your real database pool here.
    pool: DatabasePool,
}

// To make our context usable by Juniper, we have to implement a marker trait.
impl juniper::Context for Context {}

struct Query;

#[graphql_object(
    // Here we specify the context type for the object.
    // We need to do this in every type that
    // needs access to the context.
    context = Context,
)]
impl Query {
    fn apiVersion() -> &str {
        "1.0"
    }

    // Arguments to resolvers can either be simple types or input objects.
    // To gain access to the context, we specify a argument
    // that is a reference to the Context type.
    // Juniper automatically injects the correct context here.
    fn human(context: &Context, id: String) -> FieldResult<Human> {
        // Get a db connection.
        let connection = context.pool.get_connection()?;
        // Execute a db query.
        // Note the use of `?` to propagate errors.
        let human = connection.find_human(&id)?;
        // Return the result.
        Ok(human)
    }
}

// Now, we do the same for our Mutation type.

struct Mutation;

#[graphql_object(
    context = Context,

    // If we need to use `ScalarValue` parametrization explicitly somewhere
    // in the object definition (like here in `FieldResult`), we should
    // declare an explicit type parameter for that, and specify it.
    scalar = S,
)]
impl<S: ScalarValue + Display> Mutation {
    fn createHuman(context: &Context, new_human: NewHuman) -> FieldResult<Human, S> {
        let db = context.pool.get_connection().map_err(|e| e.map_scalar_value())?;
        let human: Human = db.insert_human(&new_human).map_err(|e| e.map_scalar_value())?;
        Ok(human)
    }
}

// A root schema consists of a query, a mutation, and a subscription.
// Request queries can be executed against a RootNode.
type Schema = juniper::RootNode<'static, Query, Mutation, EmptySubscription<Context>>;
#
# fn main() {
#   let _ = Schema::new(Query, Mutation{}, EmptySubscription::new());
# }
```

We now have a very simple but functional schema for a GraphQL server!

To actually serve the schema, see the guides for our various [server integrations](./servers/index.md).

Juniper is a library that can be used in many contexts--it does not require a server and it does not have a dependency on a particular transport or serialization format. You can invoke the executor directly to get a result for a query:

## Executor

You can invoke `juniper::execute` directly to run a GraphQL query:

```rust
# // Only needed due to 2018 edition because the macro is not accessible.
# #[macro_use] extern crate juniper;
use juniper::{
    graphql_object, EmptyMutation, EmptySubscription, FieldResult, 
    GraphQLEnum, Variables, graphql_value,
};

#[derive(GraphQLEnum, Clone, Copy)]
enum Episode {
    NewHope,
    Empire,
    Jedi,
}

// Arbitrary context data.
struct Ctx(Episode);

impl juniper::Context for Ctx {}

struct Query;

#[graphql_object(context = Ctx)]
impl Query {
    fn favoriteEpisode(context: &Ctx) -> FieldResult<Episode> {
        Ok(context.0)
    }
}

// A root schema consists of a query, a mutation, and a subscription.
// Request queries can be executed against a RootNode.
type Schema = juniper::RootNode<'static, Query, EmptyMutation<Ctx>, EmptySubscription<Ctx>>;

fn main() {
    // Create a context object.
    let ctx = Ctx(Episode::NewHope);

    // Run the executor.
    let (res, _errors) = juniper::execute_sync(
        "query { favoriteEpisode }",
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
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

[juniper-from-schema]: https://github.com/davidpdrsn/juniper-from-schema
[schema_approach]: https://blog.logrocket.com/code-first-vs-schema-first-development-graphql/
[hyper]: servers/hyper.md
[warp]: servers/warp.md
[rocket]: servers/rocket.md
[iron]: servers/iron.md
[tutorial]: ./tutorial.html
[graphql_object]: https://docs.rs/juniper/latest/juniper/macro.graphql_object.html
