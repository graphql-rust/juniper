Quickstart
==========

This page will give you a short introduction to the concepts in [Juniper].

**[Juniper] follows a [code-first] approach to define a [GraphQL] schema.**

> **TIP**: For a [schema-first] approach, consider using a [`juniper-from-schema`] crate for generating a [`juniper`]-based code from a [schema] file.




## Installation

```toml
[dependencies]
juniper = "0.17.1"
```




## Schema

Exposing simple enums and structs as [GraphQL] types is just a matter of adding a custom [derive attribute] to them. [Juniper] includes support for basic [Rust] types that naturally map to [GraphQL] features, such as `Option<T>`, `Vec<T>`, `Box<T>`, `Arc<T>`, `String`, `f64`, `i32`, references, slices and arrays.

For more advanced mappings, [Juniper] provides multiple macros to map your [Rust] types to a [GraphQL schema][schema]. The most important one is the [`#[graphql_object]` attribute][2] that is used for declaring a [GraphQL object] with resolvers (typically used for declaring [`Query` and `Mutation` roots][1]).

```rust
# #![expect(unused_variables, reason = "example")]
# extern crate juniper;
#
# use std::fmt::Display;
#
use juniper::{
    EmptySubscription, FieldResult, GraphQLEnum, GraphQLInputObject, 
    GraphQLObject, ScalarValue, graphql_object,
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

// Now, we create our root `Query` and `Mutation` types with resolvers by using 
// the `#[graphql_object]` attribute.

// Resolvers can have a context that allows accessing shared state like a 
// database pool.
struct Context {
    // Use your real database pool here.
    db: DatabasePool,
}

// To make our `Context` usable by `juniper`, we have to implement a marker 
// trait.
impl juniper::Context for Context {}

struct Query;

// Here we specify the context type for the object.
// We need to do this in every type that needs access to the `Context`.
#[graphql_object]
#[graphql(context = Context)]
impl Query {
    // Note, that the field name will be automatically converted to the
    // `camelCased` variant, just as GraphQL conventions imply.
    fn api_version() -> &'static str {
        "1.0"
    }

    fn human(
        // Arguments to resolvers can either be simple scalar types, enums or 
        // input objects.
        id: String,
        // To gain access to the `Context`, we specify a `context`-named 
        // argument referring the correspondent `Context` type, and `juniper`
        // will inject it automatically.
        context: &Context,
    ) -> FieldResult<Human> {
        // Get a `db` connection.
        let conn = context.db.get_connection()?;
        // Execute a `db` query.
        // Note the use of `?` to propagate errors.
        let human = conn.find_human(&id)?;
        // Return the result.
        Ok(human)
    }
}

// Now, we do the same for our `Mutation` type.

struct Mutation;

#[graphql_object]
#[graphql(
    context = Context,
    // If we need to use `ScalarValue` parametrization explicitly somewhere
    // in the object definition (like here in `FieldResult`), we could
    // declare an explicit type parameter for that, and specify it.
    scalar = S: ScalarValue + Display,
)]
impl Mutation {
    fn create_human<S: ScalarValue + Display>(
        new_human: NewHuman,
        context: &Context,
    ) -> FieldResult<Human, S> {
        let db = context.db.get_connection().map_err(|e| e.map_scalar_value())?;
        let human: Human = db.insert_human(&new_human).map_err(|e| e.map_scalar_value())?;
        Ok(human)
    }
}

// Root schema consists of a query, a mutation, and a subscription.
// Request queries can be executed against a `RootNode`.
type Schema = juniper::RootNode<Query, Mutation, EmptySubscription<Context>>;
#
# fn main() {
#     _ = Schema::new(Query, Mutation, EmptySubscription::new());
# }
```

Now we have a very simple but functional schema for a [GraphQL] server!

To actually serve the [schema], see the guides for our various [server integrations](serve/index.md).




## Execution

[Juniper] is a library that can be used in many contexts: it doesn't require a server, nor it has a dependency on a particular transport or serialization format. You can invoke the `juniper::execute()` directly to get a result for a [GraphQL] query:

```rust
# // Only needed due to 2018 edition because the macro is not accessible.
# #[macro_use] extern crate juniper;
use juniper::{
    EmptyMutation, EmptySubscription, GraphQLEnum, Variables,
    graphql_object, graphql_value,
};

#[derive(GraphQLEnum, Clone, Copy)]
enum Episode {
    // Note, that the enum value will be automatically converted to the
    // `SCREAMING_SNAKE_CASE` variant, just as GraphQL conventions imply.
    NewHope,
    Empire,
    Jedi,
}

// Arbitrary context data.
struct Ctx(Episode);

impl juniper::Context for Ctx {}

struct Query;

#[graphql_object]
#[graphql(context = Ctx)]
impl Query {
    fn favorite_episode(context: &Ctx) -> Episode {
        context.0
    }
}

type Schema = juniper::RootNode<Query, EmptyMutation<Ctx>, EmptySubscription<Ctx>>;

fn main() {
    // Create a context.
    let ctx = Ctx(Episode::NewHope);

    // Run the execution.
    let (res, _errors) = juniper::execute_sync(
        "query { favoriteEpisode }",
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &ctx,
    ).unwrap();

    assert_eq!(
        res,
        graphql_value!({
            "favoriteEpisode": "NEW_HOPE",
        }),
    );
}
```




[`juniper`]: https://docs.rs/juniper
[`juniper-from-schema`]: https://docs.rs/juniper-from-schema
[code-first]: https://www.apollographql.com/blog/backend/architecture/schema-first-vs-code-only-graphql#code-only
[derive attribute]: https://doc.rust-lang.org/stable/reference/attributes/derive.html#derive
[GraphQL]: https://graphql.org
[GraphQL object]: https://spec.graphql.org/October2021#sec-Objects
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org
[schema]: https://graphql.org/learn/schema
[schema-first]: https://www.apollographql.com/blog/backend/architecture/schema-first-vs-code-only-graphql#schema-first

[1]: https://spec.graphql.org/October2021#sec-Root-Operation-Types
[2]: https://docs.rs/juniper/0.17.1/juniper/macro.graphql_object.html
