//! [`juniper`] engines.
//!
//! The engines serve the same schema with two kinds of resolvers. The [`sync_resolvers`] schema
//! has synchronous resolvers. Synchronous resolvers are the usual style for `juniper`. The
//! [`async_resolvers`] schema has `async fn` resolvers on all fields. `async-graphql` supports
//! only this style.

pub mod async_resolvers;
pub mod sync_resolvers;

use futures::executor::block_on;
use juniper::{EmptyMutation, EmptySubscription, GraphQLTypeAsync, RootNode, http::GraphQLRequest};

use super::Engine;

/// [`juniper`] engine with the [`sync_resolvers`] schema.
///
/// Executes requests with [`GraphQLRequest::execute()`] if `ASYNC` is `true`, and with
/// [`GraphQLRequest::execute_sync()`] otherwise.
pub struct Juniper<const ASYNC: bool> {
    schema: Schema<sync_resolvers::Query>,
}

/// [`Juniper`] engine with asynchronous execution.
pub type JuniperAsync = Juniper<true>;

/// [`Juniper`] engine with synchronous execution.
pub type JuniperSync = Juniper<false>;

impl<const ASYNC: bool> Default for Juniper<ASYNC> {
    fn default() -> Self {
        Self {
            schema: Schema::new(
                sync_resolvers::Query,
                EmptyMutation::new(),
                EmptySubscription::new(),
            ),
        }
    }
}

impl<const ASYNC: bool> Engine for Juniper<ASYNC> {
    const NAME: &'static str = if ASYNC { "juniper" } else { "juniper_sync" };

    fn execute(&self, request: &str) -> String {
        if ASYNC {
            execute_async(&self.schema, request)
        } else {
            let request: GraphQLRequest = serde_json::from_str(request).expect("valid request");
            let response = request.execute_sync(&self.schema, &());
            serde_json::to_string(&response).expect("response serializes to JSON")
        }
    }
}

/// [`juniper`] engine with the [`async_resolvers`] schema and asynchronous execution.
pub struct JuniperAsyncFn {
    schema: Schema<async_resolvers::Query>,
}

impl Default for JuniperAsyncFn {
    fn default() -> Self {
        Self {
            schema: Schema::new(
                async_resolvers::Query,
                EmptyMutation::new(),
                EmptySubscription::new(),
            ),
        }
    }
}

impl Engine for JuniperAsyncFn {
    const NAME: &'static str = "juniper_async_fn";

    fn execute(&self, request: &str) -> String {
        execute_async(&self.schema, request)
    }
}

type Schema<Query> = RootNode<Query, EmptyMutation, EmptySubscription>;

/// Executes the JSON `request` body with [`GraphQLRequest::execute()`] and returns the JSON
/// response body.
fn execute_async<Query>(schema: &Schema<Query>, request: &str) -> String
where
    Query: GraphQLTypeAsync<Context = (), TypeInfo = ()>,
{
    let request: GraphQLRequest = serde_json::from_str(request).expect("valid request");
    let response = block_on(request.execute(schema, &()));
    serde_json::to_string(&response).expect("response serializes to JSON")
}

/// Defines the types of the benchmark schema in the calling module.
///
/// The `async` argument makes all resolvers `async fn`. One definition keeps the schemas with
/// synchronous and asynchronous resolvers the same.
macro_rules! schema {
    ($($asyncness:tt)?) => {
        use juniper::{graphql_interface, graphql_object};

        use crate::data::{self, Episode, Role};

        /// Root query type.
        pub struct Query;

        #[graphql_object]
        impl Query {
            $($asyncness)? fn hello() -> &'static str {
                "world"
            }

            $($asyncness)? fn users(first: i32) -> Vec<User> {
                data::first_users(first).map(User).collect()
            }

            $($asyncness)? fn hero(episode: Option<Episode>) -> CharacterValue {
                data::hero(episode).into()
            }

            $($asyncness)? fn humans() -> Vec<Human> {
                data::HUMANS.into_iter().map(Human).collect()
            }

            $($asyncness)? fn droids() -> Vec<Droid> {
                data::DROIDS.into_iter().map(Droid).collect()
            }
        }

        struct User(&'static data::User);

        #[graphql_object]
        impl User {
            $($asyncness)? fn id(&self) -> i32 {
                self.0.id
            }

            $($asyncness)? fn name(&self) -> &str {
                &self.0.name
            }

            $($asyncness)? fn email(&self) -> &str {
                &self.0.email
            }

            $($asyncness)? fn active(&self) -> bool {
                self.0.active
            }

            $($asyncness)? fn score(&self) -> f64 {
                self.0.score
            }

            $($asyncness)? fn role(&self) -> Role {
                self.0.role
            }

            $($asyncness)? fn friends(&self, first: i32) -> Vec<User> {
                self.0.friends(first).map(User).collect()
            }
        }

        #[graphql_interface(for = [Human, Droid])]
        #[expect(dead_code, reason = "GraphQL schema definition")]
        trait Character {
            $($asyncness)? fn id(&self) -> &str;

            $($asyncness)? fn name(&self) -> &str;

            $($asyncness)? fn friends(&self) -> Vec<CharacterValue>;

            $($asyncness)? fn appears_in(&self) -> &[Episode];
        }

        impl From<data::Character> for CharacterValue {
            fn from(character: data::Character) -> Self {
                match character {
                    data::Character::Human(human) => Human(human).into(),
                    data::Character::Droid(droid) => Droid(droid).into(),
                }
            }
        }

        struct Human(&'static data::Human);

        #[graphql_object(impl = CharacterValue)]
        impl Human {
            $($asyncness)? fn id(&self) -> &str {
                self.0.id
            }

            $($asyncness)? fn name(&self) -> &str {
                self.0.name
            }

            $($asyncness)? fn friends(&self) -> Vec<CharacterValue> {
                self.0.friends.iter().copied().map(Into::into).collect()
            }

            $($asyncness)? fn appears_in(&self) -> &[Episode] {
                self.0.appears_in
            }

            $($asyncness)? fn home_planet(&self) -> Option<&str> {
                self.0.home_planet
            }
        }

        struct Droid(&'static data::Droid);

        #[graphql_object(impl = CharacterValue)]
        impl Droid {
            $($asyncness)? fn id(&self) -> &str {
                self.0.id
            }

            $($asyncness)? fn name(&self) -> &str {
                self.0.name
            }

            $($asyncness)? fn friends(&self) -> Vec<CharacterValue> {
                self.0.friends.iter().copied().map(Into::into).collect()
            }

            $($asyncness)? fn appears_in(&self) -> &[Episode] {
                self.0.appears_in
            }

            $($asyncness)? fn primary_function(&self) -> &str {
                self.0.primary_function
            }
        }
    };
}

use schema;
