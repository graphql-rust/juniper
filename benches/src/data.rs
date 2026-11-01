//! Data served by every engine.
//!
//! All data is `'static`. Resolvers only read fields and follow references, so the benchmarks
//! measure the engines and not the data access.

use std::sync::LazyLock;

/// Number of users in [`users()`].
pub const USER_COUNT: usize = 1_000;

/// User of a social network.
#[derive(Debug)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub score: f64,
    pub role: Role,
}

impl User {
    fn new(id: i32) -> Self {
        let roles = [Role::Admin, Role::Moderator, Role::Member, Role::Guest];
        Self {
            id,
            name: format!("User {id}"),
            email: format!("user{id}@example.com"),
            active: id % 2 == 0,
            score: f64::from(id) / 2.0,
            role: roles[id.unsigned_abs() as usize % roles.len()],
        }
    }

    /// Returns up to `count` users that follow this one in [`users()`].
    ///
    /// The sequence wraps around to the start of [`users()`]. A negative `count` returns no users.
    pub fn friends(&self, count: i32) -> impl Iterator<Item = &'static User> {
        let all = users();
        let start = self.id.unsigned_abs() as usize + 1;
        let count = usize::try_from(count).unwrap_or_default();
        (start..start + count).map(move |index| &all[index % all.len()])
    }
}

/// Access level of a [`User`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, async_graphql::Enum, juniper::GraphQLEnum)]
pub enum Role {
    Admin,
    Moderator,
    Member,
    Guest,
}

/// Returns all users, ordered by ID from `0`.
pub fn users() -> &'static [User] {
    static USERS: LazyLock<Vec<User>> = LazyLock::new(|| {
        let count = i32::try_from(USER_COUNT).expect("`USER_COUNT` fits in `i32`");
        (0..count).map(User::new).collect()
    });
    &USERS
}

/// Returns up to `count` first users. A negative `count` returns no users.
pub fn first_users(count: i32) -> impl Iterator<Item = &'static User> {
    users()
        .iter()
        .take(usize::try_from(count).unwrap_or_default())
}

/// Film of the original Star Wars trilogy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, async_graphql::Enum, juniper::GraphQLEnum)]
pub enum Episode {
    NewHope,
    Empire,
    Jedi,
}

/// Character of the Star Wars films.
#[derive(Clone, Copy, Debug)]
pub enum Character {
    Human(&'static Human),
    Droid(&'static Droid),
}

/// Human character of the Star Wars films.
#[derive(Debug)]
pub struct Human {
    pub id: &'static str,
    pub name: &'static str,
    pub friends: &'static [Character],
    pub appears_in: &'static [Episode],
    pub home_planet: Option<&'static str>,
}

/// Droid character of the Star Wars films.
#[derive(Debug)]
pub struct Droid {
    pub id: &'static str,
    pub name: &'static str,
    pub friends: &'static [Character],
    pub appears_in: &'static [Episode],
    pub primary_function: &'static str,
}

/// Human characters in the order of their IDs.
pub static HUMANS: [&Human; 5] = [&LUKE, &VADER, &HAN, &LEIA, &TARKIN];

/// Droid characters in the order of their IDs.
pub static DROIDS: [&Droid; 2] = [&THREEPIO, &ARTOO];

/// Returns the hero of `episode`, or the hero of the whole saga if `episode` is [`None`].
pub fn hero(episode: Option<Episode>) -> Character {
    if episode == Some(Episode::Empire) {
        Character::Human(&LUKE)
    } else {
        Character::Droid(&ARTOO)
    }
}

const ALL_EPISODES: &[Episode] = &[Episode::NewHope, Episode::Empire, Episode::Jedi];

static LUKE: Human = Human {
    id: "1000",
    name: "Luke Skywalker",
    friends: &[
        Character::Human(&HAN),
        Character::Human(&LEIA),
        Character::Droid(&THREEPIO),
        Character::Droid(&ARTOO),
    ],
    appears_in: ALL_EPISODES,
    home_planet: Some("Tatooine"),
};

static VADER: Human = Human {
    id: "1001",
    name: "Darth Vader",
    friends: &[Character::Human(&TARKIN)],
    appears_in: ALL_EPISODES,
    home_planet: Some("Tatooine"),
};

static HAN: Human = Human {
    id: "1002",
    name: "Han Solo",
    friends: &[
        Character::Human(&LUKE),
        Character::Human(&LEIA),
        Character::Droid(&ARTOO),
    ],
    appears_in: ALL_EPISODES,
    home_planet: None,
};

static LEIA: Human = Human {
    id: "1003",
    name: "Leia Organa",
    friends: &[
        Character::Human(&LUKE),
        Character::Human(&HAN),
        Character::Droid(&THREEPIO),
        Character::Droid(&ARTOO),
    ],
    appears_in: ALL_EPISODES,
    home_planet: Some("Alderaan"),
};

static TARKIN: Human = Human {
    id: "1004",
    name: "Wilhuff Tarkin",
    friends: &[Character::Human(&VADER)],
    appears_in: &[Episode::NewHope],
    home_planet: None,
};

static THREEPIO: Droid = Droid {
    id: "2000",
    name: "C-3PO",
    friends: &[
        Character::Human(&LUKE),
        Character::Human(&HAN),
        Character::Human(&LEIA),
        Character::Droid(&ARTOO),
    ],
    appears_in: ALL_EPISODES,
    primary_function: "Protocol",
};

static ARTOO: Droid = Droid {
    id: "2001",
    name: "R2-D2",
    friends: &[
        Character::Human(&LUKE),
        Character::Human(&HAN),
        Character::Human(&LEIA),
    ],
    appears_in: ALL_EPISODES,
    primary_function: "Astromech",
};
