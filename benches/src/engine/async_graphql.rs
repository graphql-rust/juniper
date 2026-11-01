//! [`async_graphql`] engine.

use async_graphql::{EmptyMutation, EmptySubscription, Interface, Object, Request, Schema};
use futures::executor::block_on;

use super::Engine;
use crate::data::{self, Episode, Role};

/// [`async_graphql`] engine.
pub struct AsyncGraphql {
    schema: Schema<Query, EmptyMutation, EmptySubscription>,
}

impl Default for AsyncGraphql {
    fn default() -> Self {
        Self {
            schema: Schema::new(Query, EmptyMutation, EmptySubscription),
        }
    }
}

impl Engine for AsyncGraphql {
    const NAME: &'static str = "async_graphql";

    fn execute(&self, request: &str) -> String {
        let request: Request = serde_json::from_str(request).expect("valid request");
        let response = block_on(self.schema.execute(request));
        serde_json::to_string(&response).expect("response serializes to JSON")
    }
}

/// Root query type.
pub struct Query;

#[Object]
impl Query {
    async fn hello(&self) -> &str {
        "world"
    }

    async fn users(&self, first: i32) -> Vec<User> {
        data::first_users(first).map(User).collect()
    }

    async fn hero(&self, episode: Option<Episode>) -> Character {
        data::hero(episode).into()
    }

    async fn humans(&self) -> Vec<Human> {
        data::HUMANS.into_iter().map(Human).collect()
    }

    async fn droids(&self) -> Vec<Droid> {
        data::DROIDS.into_iter().map(Droid).collect()
    }
}

struct User(&'static data::User);

#[Object]
impl User {
    async fn id(&self) -> i32 {
        self.0.id
    }

    async fn name(&self) -> &str {
        &self.0.name
    }

    async fn email(&self) -> &str {
        &self.0.email
    }

    async fn active(&self) -> bool {
        self.0.active
    }

    async fn score(&self) -> f64 {
        self.0.score
    }

    async fn role(&self) -> Role {
        self.0.role
    }

    async fn friends(&self, first: i32) -> Vec<User> {
        self.0.friends(first).map(User).collect()
    }
}

#[derive(Interface)]
#[expect(
    clippy::duplicated_attributes,
    reason = "fields of the same type repeat `ty`"
)]
#[graphql(
    field(name = "id", ty = "&str"),
    field(name = "name", ty = "&str"),
    field(name = "friends", ty = "Vec<Character>"),
    field(name = "appears_in", ty = "&[Episode]")
)]
enum Character {
    Human(Human),
    Droid(Droid),
}

impl From<data::Character> for Character {
    fn from(character: data::Character) -> Self {
        match character {
            data::Character::Human(human) => Self::Human(Human(human)),
            data::Character::Droid(droid) => Self::Droid(Droid(droid)),
        }
    }
}

struct Human(&'static data::Human);

#[Object]
impl Human {
    async fn id(&self) -> &str {
        self.0.id
    }

    async fn name(&self) -> &str {
        self.0.name
    }

    async fn friends(&self) -> Vec<Character> {
        self.0.friends.iter().copied().map(Into::into).collect()
    }

    async fn appears_in(&self) -> &[Episode] {
        self.0.appears_in
    }

    async fn home_planet(&self) -> Option<&str> {
        self.0.home_planet
    }
}

struct Droid(&'static data::Droid);

#[Object]
impl Droid {
    async fn id(&self) -> &str {
        self.0.id
    }

    async fn name(&self) -> &str {
        self.0.name
    }

    async fn friends(&self) -> Vec<Character> {
        self.0.friends.iter().copied().map(Into::into).collect()
    }

    async fn appears_in(&self) -> &[Episode] {
        self.0.appears_in
    }

    async fn primary_function(&self) -> &str {
        self.0.primary_function
    }
}
