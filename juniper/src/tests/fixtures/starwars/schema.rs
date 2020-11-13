#![allow(missing_docs)]

use std::{collections::HashMap, pin::Pin};

use crate::{graphql_interface, graphql_object, graphql_subscription, Context, GraphQLEnum};

pub struct Query;

#[graphql_object(context = Database)]
/// The root query object of the schema
impl Query {
    #[graphql(arguments(id(description = "id of the human")))]
    fn human(database: &Database, id: String) -> Option<&Human> {
        database.get_human(&id)
    }

    #[graphql(arguments(id(description = "id of the droid")))]
    fn droid(database: &Database, id: String) -> Option<&Droid> {
        database.get_droid(&id)
    }

    #[graphql(arguments(episode(
        description = "If omitted, returns the hero of the whole saga. \
                       If provided, returns the hero of that particular episode"
    )))]
    fn hero(database: &Database, episode: Option<Episode>) -> Option<CharacterValue> {
        Some(database.get_hero(episode))
    }
}

pub struct Subscription;

type HumanStream = Pin<Box<dyn futures::Stream<Item = Human> + Send>>;

#[graphql_subscription(context = Database)]
/// Super basic subscription fixture
impl Subscription {
    async fn async_human(context: &Database) -> HumanStream {
        let human = context.get_human("1000").unwrap().clone();
        Box::pin(futures::stream::once(futures::future::ready(human)))
    }
}
#[derive(GraphQLEnum, Clone, Copy, Debug, Eq, PartialEq)]
pub enum Episode {
    #[graphql(name = "NEW_HOPE")]
    NewHope,
    Empire,
    Jedi,
}

#[graphql_interface(for = [Human, Droid], context = Database)]
/// A character in the Star Wars Trilogy
pub trait Character {
    /// The id of the character
    fn id(&self) -> &str;

    /// The name of the character
    fn name(&self) -> Option<&str>;

    /// The friends of the character
    fn friends(&self, ctx: &Database) -> Vec<CharacterValue>;

    /// Which movies they appear in
    fn appears_in(&self) -> &[Episode];

    #[graphql(ignore)]
    fn friends_ids(&self) -> &[String];
}

#[derive(Clone)]
pub struct Human {
    id: String,
    name: String,
    friend_ids: Vec<String>,
    appears_in: Vec<Episode>,
    secret_backstory: Option<String>,
    home_planet: Option<String>,
}

impl Human {
    pub fn new(
        id: &str,
        name: &str,
        friend_ids: &[&str],
        appears_in: &[Episode],
        secret_backstory: Option<&str>,
        home_planet: Option<&str>,
    ) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            friend_ids: friend_ids
                .to_owned()
                .into_iter()
                .map(ToOwned::to_owned)
                .collect(),
            appears_in: appears_in.to_vec(),
            secret_backstory: secret_backstory.map(ToOwned::to_owned),
            home_planet: home_planet.map(|p| p.to_owned()),
        }
    }
}

/// A humanoid creature in the Star Wars universe.
#[graphql_object(context = Database, impl = CharacterValue)]
impl Human {
    /// The id of the human
    fn id(&self) -> &str {
        &self.id
    }

    /// The name of the human
    fn name(&self) -> Option<&str> {
        Some(self.name.as_str())
    }

    /// The friends of the human
    fn friends(&self, ctx: &Database) -> Vec<CharacterValue> {
        ctx.get_friends(self)
    }

    /// Which movies they appear in
    fn appears_in(&self) -> &[Episode] {
        &self.appears_in
    }

    /// The home planet of the human
    fn home_planet(&self) -> &Option<String> {
        &self.home_planet
    }
}

#[graphql_interface]
impl Character for Human {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> Option<&str> {
        Some(&self.name)
    }

    fn friends(&self, ctx: &Database) -> Vec<CharacterValue> {
        ctx.get_friends(self)
    }

    fn appears_in(&self) -> &[Episode] {
        &self.appears_in
    }

    fn friends_ids(&self) -> &[String] {
        &self.friend_ids
    }
}

#[derive(Clone)]
pub struct Droid {
    id: String,
    name: String,
    friend_ids: Vec<String>,
    appears_in: Vec<Episode>,
    secret_backstory: Option<String>,
    primary_function: Option<String>,
}

impl Droid {
    pub fn new(
        id: &str,
        name: &str,
        friend_ids: &[&str],
        appears_in: &[Episode],
        secret_backstory: Option<&str>,
        primary_function: Option<&str>,
    ) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            friend_ids: friend_ids
                .to_owned()
                .into_iter()
                .map(ToOwned::to_owned)
                .collect(),
            appears_in: appears_in.to_vec(),
            secret_backstory: secret_backstory.map(ToOwned::to_owned),
            primary_function: primary_function.map(ToOwned::to_owned),
        }
    }
}

/// A mechanical creature in the Star Wars universe.
#[graphql_object(context = Database, impl = CharacterValue)]
impl Droid {
    /// The id of the droid
    fn id(&self) -> &str {
        &self.id
    }

    /// The name of the droid
    fn name(&self) -> Option<&str> {
        Some(self.name.as_str())
    }

    /// The friends of the droid
    fn friends(&self, ctx: &Database) -> Vec<CharacterValue> {
        ctx.get_friends(self)
    }

    /// Which movies they appear in
    fn appears_in(&self) -> &[Episode] {
        &self.appears_in
    }

    /// The primary function of the droid
    fn primary_function(&self) -> &Option<String> {
        &self.primary_function
    }
}

#[graphql_interface]
impl Character for Droid {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> Option<&str> {
        Some(&self.name)
    }

    fn friends(&self, ctx: &Database) -> Vec<CharacterValue> {
        ctx.get_friends(self)
    }

    fn appears_in(&self) -> &[Episode] {
        &self.appears_in
    }

    fn friends_ids(&self) -> &[String] {
        &self.friend_ids
    }
}

#[derive(Default, Clone)]
pub struct Database {
    humans: HashMap<String, Human>,
    droids: HashMap<String, Droid>,
}

impl Context for Database {}

impl Database {
    pub fn new() -> Database {
        let mut humans = HashMap::new();
        let mut droids = HashMap::new();

        humans.insert(
            "1000".to_owned(),
            Human::new(
                "1000",
                "Luke Skywalker",
                &["1002", "1003", "2000", "2001"],
                &[Episode::NewHope, Episode::Empire, Episode::Jedi],
                None,
                Some("Tatooine"),
            ),
        );

        humans.insert(
            "1001".to_owned(),
            Human::new(
                "1001",
                "Darth Vader",
                &["1004"],
                &[Episode::NewHope, Episode::Empire, Episode::Jedi],
                None,
                Some("Tatooine"),
            ),
        );

        humans.insert(
            "1002".to_owned(),
            Human::new(
                "1002",
                "Han Solo",
                &["1000", "1003", "2001"],
                &[Episode::NewHope, Episode::Empire, Episode::Jedi],
                None,
                None,
            ),
        );

        humans.insert(
            "1003".to_owned(),
            Human::new(
                "1003",
                "Leia Organa",
                &["1000", "1002", "2000", "2001"],
                &[Episode::NewHope, Episode::Empire, Episode::Jedi],
                None,
                Some("Alderaan"),
            ),
        );

        humans.insert(
            "1004".to_owned(),
            Human::new(
                "1004",
                "Wilhuff Tarkin",
                &["1001"],
                &[Episode::NewHope],
                None,
                None,
            ),
        );

        droids.insert(
            "2000".to_owned(),
            Droid::new(
                "2000",
                "C-3PO",
                &["1000", "1002", "1003", "2001"],
                &[Episode::NewHope, Episode::Empire, Episode::Jedi],
                None,
                Some("Protocol"),
            ),
        );

        droids.insert(
            "2001".to_owned(),
            Droid::new(
                "2001",
                "R2-D2",
                &["1000", "1002", "1003"],
                &[Episode::NewHope, Episode::Empire, Episode::Jedi],
                None,
                Some("Astromech"),
            ),
        );

        Database { humans, droids }
    }

    pub fn get_hero(&self, episode: Option<Episode>) -> CharacterValue {
        if episode == Some(Episode::Empire) {
            self.get_human("1000").unwrap().clone().into()
        } else {
            self.get_droid("2001").unwrap().clone().into()
        }
    }

    pub fn get_human(&self, id: &str) -> Option<&Human> {
        self.humans.get(id)
    }

    pub fn get_droid(&self, id: &str) -> Option<&Droid> {
        self.droids.get(id)
    }

    pub fn get_character(&self, id: &str) -> Option<CharacterValue> {
        if let Some(h) = self.humans.get(id) {
            Some(h.clone().into())
        } else if let Some(d) = self.droids.get(id) {
            Some(d.clone().into())
        } else {
            None
        }
    }

    pub fn get_friends(&self, c: &dyn Character) -> Vec<CharacterValue> {
        c.friends_ids()
            .iter()
            .flat_map(|id| self.get_character(id))
            .collect()
    }
}
