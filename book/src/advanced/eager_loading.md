Eager loading
=============

As a further evolution of the [dealing with the N+1 problem via look-ahead](lookahead.md#n1-problem), we may systematically remodel [Rust] types mapping to [GraphQL] ones in the way to encourage doing eager preloading of data for its [fields][0] and using the already preloaded data when resolving a particular [field][0].

At the moment, this approach is represented with the [`juniper-eager-loading`] crate for [Juniper].

> **NOTE**: Since this library requires [`juniper-from-schema`], it's best first to become familiar with it.

<!-- TODO: Provide example of solving the problem from "N+1 chapter" once `juniper-eager-loading` support the latest `juniper`. -->

From ["How this library works at a high level"][11] and ["A real example"][12] sections of [`juniper-eager-loading`] documentation:

> ### How this library works at a high level
>
> If you have a GraphQL type like this
>
> ```graphql
> type User {
>     id: Int!
>     country: Country!
> }
> ```
>
> You might create the corresponding Rust model type like this:
>
> ```rust
> struct User {
>     id: i32,
>     country_id: i32,
> }
> ```
>
> However this approach has one big issue. How are you going to resolve the field `User.country`
> without doing a database query? All the resolver has access to is a `User` with a `country_id`
> field. It can't get the country without loading it from the database...
>
> Fundamentally these kinds of model structs don't work for eager loading with GraphQL. So
> this library takes a different approach.
>
> What if we created separate structs for the database models and the GraphQL models? Something
> like this:
>
> ```rust
> # fn main() {}
> #
> mod models {
>     pub struct User {
>         id: i32,
>         country_id: i32
>     }
>
>     pub struct Country {
>         id: i32,
>     }
> }
>
> struct User {
>     user: models::User,
>     country: HasOne<Country>,
> }
>
> struct Country {
>     country: models::Country
> }
>
> enum HasOne<T> {
>     Loaded(T),
>     NotLoaded,
> }
> ```
>
> Now we're able to resolve the query with code like this:
>
> 1. Load all the users (first query).
> 2. Map the users to a list of country ids.
> 3. Load all the countries with those ids (second query).
> 4. Pair up the users with the country with the correct id, so change `User.country` from
>    `HasOne::NotLoaded` to `HasOne::Loaded(matching_country)`.
> 5. When resolving the GraphQL field `User.country` simply return the loaded country.
>
> ### A real example
>
> ```rust,ignore
> use juniper::{Executor, FieldResult};
> use juniper_eager_loading::{prelude::*, EagerLoading, HasOne};
> use juniper_from_schema::graphql_schema;
> use std::error::Error;
>
> // Define our GraphQL schema.
> graphql_schema! {
>     schema {
>         query: Query
>     }
>
>     type Query {
>         allUsers: [User!]! @juniper(ownership: "owned")
>     }
>
>     type User {
>         id: Int!
>         country: Country!
>     }
>
>     type Country {
>         id: Int!
>     }
> }
>
> // Our model types.
> mod models {
>     use std::error::Error;
>     use juniper_eager_loading::LoadFrom;
>
>     #[derive(Clone)]
>     pub struct User {
>         pub id: i32,
>         pub country_id: i32
>     }
>
>     #[derive(Clone)]
>     pub struct Country {
>         pub id: i32,
>     }
>
>     // This trait is required for eager loading countries.
>     // It defines how to load a list of countries from a list of ids.
>     // Notice that `Context` is generic and can be whatever you want.
>     // It will normally be your Juniper context which would contain
>     // a database connection.
>     impl LoadFrom<i32> for Country {
>         type Error = Box<dyn Error>;
>         type Context = super::Context;
>
>         fn load(
>             employments: &[i32],
>             field_args: &(),
>             ctx: &Self::Context,
>         ) -> Result<Vec<Self>, Self::Error> {
>             // ...
>             # unimplemented!()
>         }
>     }
> }
>
> // Our sample database connection type.
> pub struct DbConnection;
>
> impl DbConnection {
>     // Function that will load all the users.
>     fn load_all_users(&self) -> Vec<models::User> {
>         // ...
>         # unimplemented!()
>     }
> }
>
> // Our Juniper context type which contains a database connection.
> pub struct Context {
>     db: DbConnection,
> }
>
> impl juniper::Context for Context {}
>
> // Our GraphQL user type.
> // `#[derive(EagerLoading)]` takes care of generating all the boilerplate code.
> #[derive(Clone, EagerLoading)]
> // You need to set the context and error type.
> #[eager_loading(
>     context = Context,
>     error = Box<dyn Error>,
>
>     // These match the default so you wouldn't have to specify them
>     model = models::User,
>     id = i32,
>     root_model_field = user,
> )]
> pub struct User {
>     // This user model is used to resolve `User.id`
>     user: models::User,
>
>     // Setup a "has one" association between a user and a country.
>     //
>     // We could also have used `#[has_one(default)]` here.
>     #[has_one(
>         foreign_key_field = country_id,
>         root_model_field = country,
>         graphql_field = country,
>     )]
>     country: HasOne<Country>,
> }
>
> // And the GraphQL country type.
> #[derive(Clone, EagerLoading)]
> #[eager_loading(context = Context, error = Box<dyn Error>)]
> pub struct Country {
>     country: models::Country,
> }
>
> // The root query GraphQL type.
> pub struct Query;
>
> impl QueryFields for Query {
>     // The resolver for `Query.allUsers`.
>     fn field_all_users(
>         &self,
>         executor: &Executor<'_, Context>,
>         trail: &QueryTrail<'_, User, Walked>,
>     ) -> FieldResult<Vec<User>> {
>         let ctx = executor.context();
>
>         // Load the model users.
>         let user_models = ctx.db.load_all_users();
>
>         // Turn the model users into GraphQL users.
>         let mut users = User::from_db_models(&user_models);
>
>         // Perform the eager loading.
>         // `trail` is used to only eager load the fields that are requested. Because
>         // we're using `QueryTrail`s from "juniper_from_schema" it would be a compile
>         // error if we eager loaded associations that aren't requested in the query.
>         User::eager_load_all_children_for_each(&mut users, &user_models, ctx, trail)?;
>
>         Ok(users)
>     }
> }
>
> impl UserFields for User {
>     fn field_id(
>         &self,
>         executor: &Executor<'_, Context>,
>     ) -> FieldResult<&i32> {
>         Ok(&self.user.id)
>     }
>
>     fn field_country(
>         &self,
>         executor: &Executor<'_, Context>,
>         trail: &QueryTrail<'_, Country, Walked>,
>     ) -> FieldResult<&Country> {
>         // This will unwrap the country from the `HasOne` or return an error if the
>         // country wasn't loaded, or wasn't found in the database.
>         Ok(self.country.try_unwrap()?)
>     }
> }
>
> impl CountryFields for Country {
>     fn field_id(
>         &self,
>         executor: &Executor<'_, Context>,
>     ) -> FieldResult<&i32> {
>         Ok(&self.country.id)
>     }
> }
> #
> # fn main() {}
> ```

For more details, check out the [`juniper-eager-loading` documentation][`juniper-eager-loading`].




## Full example

For a full example using eager loading in [Juniper] check out the [`davidpdrsn/graphql-app-example` repository][10].




[`juniper-eager-loading`]: https://docs.rs/juniper-eager-loading
[`juniper-from-schema`]: https://docs.rs/juniper-from-schema
[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[Redis]: https://redis.io
[Rust]: https://www.rust-lang.org

[0]: https://spec.graphql.org/October2021#sec-Language.Fields
[10]: https://github.com/davidpdrsn/graphql-app-example
[11]: https://docs.rs/juniper-eager-loading/latest/juniper_eager_loading#how-this-library-works-at-a-high-level
[12]: https://docs.rs/juniper-eager-loading/latest/juniper_eager_loading#a-real-example
