use executor::Context;
use tests::model::{Character, Database, Droid, Episode, Human};

impl Context for Database {}

graphql_interface!(<'a> &'a Character: Database as "Character" |&self| {
    description: "A character in the Star Wars Trilogy"

    field id() -> &str as "The id of the character" {
        self.id()
    }

    field name() -> Option<&str> as "The name of the character" {
        Some(self.name())
    }

    field friends(&executor) -> Vec<&Character>
    as "The friends of the character" {
        executor.context().get_friends(self.as_character())
    }

    field appears_in() -> &[Episode] as "Which movies they appear in" {
        self.appears_in()
    }

    instance_resolvers: |&context| {
        &Human => context.get_human(&self.id()),
        &Droid => context.get_droid(&self.id()),
    }
});

graphql_object!(<'a> &'a Human: Database as "Human" |&self| {
    description: "A humanoid creature in the Star Wars universe."

    interfaces: [&Character]

    field id() -> &str as "The id of the human"{
        self.id()
    }

    field name() -> Option<&str> as "The name of the human" {
        Some(self.name())
    }

    field friends(&executor) -> Vec<&Character>
    as "The friends of the human" {
        executor.context().get_friends(self.as_character())
    }

    field appears_in() -> &[Episode] as "Which movies they appear in" {
        self.appears_in()
    }

    field home_planet() -> &Option<String> as "The home planet of the human" {
        self.home_planet()
    }
});

graphql_object!(<'a> &'a Droid: Database as "Droid" |&self| {
    description: "A mechanical creature in the Star Wars universe."

    interfaces: [&Character]

    field id() -> &str as "The id of the droid" {
        self.id()
    }

    field name() -> Option<&str> as "The name of the droid" {
        Some(self.name())
    }

    field friends(&executor) -> Vec<&Character>
    as "The friends of the droid" {
        executor.context().get_friends(self.as_character())
    }

    field appears_in() -> &[Episode] as "Which movies they appear in" {
        self.appears_in()
    }

    field primary_function() -> &Option<String> as "The primary function of the droid" {
        self.primary_function()
    }
});

graphql_object!(Database: Database as "Query" |&self| {
    description: "The root query object of the schema"

    field human(
        id: String as "id of the human"
    ) -> Option<&Human> {
        self.get_human(&id)
    }

    field droid(
        id: String as "id of the droid"
    ) -> Option<&Droid> {
        self.get_droid(&id)
    }

    field hero(
        episode: Option<Episode> as
        "If omitted, returns the hero of the whole saga. If provided, returns \
        the hero of that particular episode"
    ) -> Option<&Character> {
        Some(self.get_hero(episode).as_character())
    }
});
