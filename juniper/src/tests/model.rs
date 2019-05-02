#![allow(missing_docs)]

use juniper_codegen::GraphQLEnumInternal as GraphQLEnum;
use std::collections::HashMap;

#[derive(GraphQLEnum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum Episode {
    #[graphql(name = "NEW_HOPE")]
    NewHope,
    Empire,
    Jedi,
}

pub trait Character {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn friend_ids(&self) -> &[String];
    fn appears_in(&self) -> &[Episode];
    fn secret_backstory(&self) -> &Option<String>;
    fn as_character(&self) -> &Character;
}

pub trait Human: Character {
    fn home_planet(&self) -> &Option<String>;
}

pub trait Droid: Character {
    fn primary_function(&self) -> &Option<String>;
}

struct HumanData {
    id: String,
    name: String,
    friend_ids: Vec<String>,
    appears_in: Vec<Episode>,
    secret_backstory: Option<String>,
    home_planet: Option<String>,
}

struct DroidData {
    id: String,
    name: String,
    friend_ids: Vec<String>,
    appears_in: Vec<Episode>,
    secret_backstory: Option<String>,
    primary_function: Option<String>,
}

impl Character for HumanData {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn friend_ids(&self) -> &[String] {
        &self.friend_ids
    }
    fn appears_in(&self) -> &[Episode] {
        &self.appears_in
    }
    fn secret_backstory(&self) -> &Option<String> {
        &self.secret_backstory
    }
    fn as_character(&self) -> &Character {
        self
    }
}

impl Human for HumanData {
    fn home_planet(&self) -> &Option<String> {
        &self.home_planet
    }
}

impl Character for DroidData {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn friend_ids(&self) -> &[String] {
        &self.friend_ids
    }
    fn appears_in(&self) -> &[Episode] {
        &self.appears_in
    }
    fn secret_backstory(&self) -> &Option<String> {
        &self.secret_backstory
    }
    fn as_character(&self) -> &Character {
        self
    }
}

impl Droid for DroidData {
    fn primary_function(&self) -> &Option<String> {
        &self.primary_function
    }
}

pub struct Database {
    humans: HashMap<String, HumanData>,
    droids: HashMap<String, DroidData>,
}

impl HumanData {
    pub fn new(
        id: &str,
        name: &str,
        friend_ids: &[&str],
        appears_in: &[Episode],
        secret_backstory: Option<&str>,
        home_planet: Option<&str>,
    ) -> HumanData {
        HumanData {
            id: id.to_owned(),
            name: name.to_owned(),
            friend_ids: friend_ids
                .to_owned()
                .into_iter()
                .map(|f| f.to_owned())
                .collect(),
            appears_in: appears_in.iter().cloned().collect(),
            secret_backstory: secret_backstory.map(|b| b.to_owned()),
            home_planet: home_planet.map(|p| p.to_owned()),
        }
    }
}

impl DroidData {
    pub fn new(
        id: &str,
        name: &str,
        friend_ids: &[&str],
        appears_in: &[Episode],
        secret_backstory: Option<&str>,
        primary_function: Option<&str>,
    ) -> DroidData {
        DroidData {
            id: id.to_owned(),
            name: name.to_owned(),
            friend_ids: friend_ids
                .to_owned()
                .into_iter()
                .map(|f| f.to_owned())
                .collect(),
            appears_in: appears_in.iter().cloned().collect(),
            secret_backstory: secret_backstory.map(|b| b.to_owned()),
            primary_function: primary_function.map(|p| p.to_owned()),
        }
    }
}

impl Database {
    pub fn new() -> Database {
        let mut humans = HashMap::new();
        let mut droids = HashMap::new();

        humans.insert(
            "1000".to_owned(),
            HumanData::new(
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
            HumanData::new(
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
            HumanData::new(
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
            HumanData::new(
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
            HumanData::new(
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
            DroidData::new(
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
            DroidData::new(
                "2001",
                "R2-D2",
                &["1000", "1002", "1003"],
                &[Episode::NewHope, Episode::Empire, Episode::Jedi],
                None,
                Some("Astromech"),
            ),
        );

        Database {
            humans: humans,
            droids: droids,
        }
    }

    pub fn get_hero(&self, episode: Option<Episode>) -> &Character {
        if episode == Some(Episode::Empire) {
            self.get_human("1000").unwrap().as_character()
        } else {
            self.get_droid("2001").unwrap().as_character()
        }
    }

    pub fn get_human(&self, id: &str) -> Option<&Human> {
        self.humans.get(id).map(|h| h as &Human)
    }

    pub fn get_droid(&self, id: &str) -> Option<&Droid> {
        self.droids.get(id).map(|d| d as &Droid)
    }

    pub fn get_character(&self, id: &str) -> Option<&Character> {
        if let Some(h) = self.humans.get(id) {
            Some(h)
        } else if let Some(d) = self.droids.get(id) {
            Some(d)
        } else {
            None
        }
    }

    pub fn get_friends(&self, c: &Character) -> Vec<&Character> {
        c.friend_ids()
            .iter()
            .flat_map(|id| self.get_character(id))
            .collect()
    }
}
