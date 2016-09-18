use executor::FieldResult;
use tests::model::{Character, Human, Droid, Database, Episode};

graphql_enum!(Episode {
    Episode::NewHope => "NEW_HOPE",
    Episode::Empire => "EMPIRE",
    Episode::Jedi => "JEDI",
});

graphql_interface!(<'a> &'a Character: Database as "Character" |&self| {
    description: "A character in the Star Wars Trilogy"

    field id() -> FieldResult<&str> as "The id of the character" {
        Ok(self.id())
    }

    field name() -> FieldResult<Option<&str>> as "The name of the character" {
        Ok(Some(self.name()))
    }

    field friends(&mut executor) -> FieldResult<Vec<&Character>>
    as "The friends of the character" {
        Ok(executor.context().get_friends(self.as_character()))
    }

    field appears_in() -> FieldResult<&[Episode]> as "Which movies they appear in" {
        Ok(self.appears_in())
    }

    instance_resolvers: |&context| [
        context.get_human(&self.id()),
        context.get_droid(&self.id()),
    ]
});

graphql_object!(<'a> &'a Human: Database as "Human" |&self| {
    description: "A humanoid creature in the Star Wars universe."

    interfaces: [&Character]

    field id() -> FieldResult<&str> as "The id of the human"{
        Ok(self.id())
    }

    field name() -> FieldResult<Option<&str>> as "The name of the human" {
        Ok(Some(self.name()))
    }

    field friends(&mut executor) -> FieldResult<Vec<&Character>>
    as "The friends of the human" {
        Ok(executor.context().get_friends(self.as_character()))
    }

    field appears_in() -> FieldResult<&[Episode]> as "Which movies they appear in" {
        Ok(self.appears_in())
    }

    field home_planet() -> FieldResult<&Option<String>> as "The home planet of the human" {
        Ok(self.home_planet())
    }
});

graphql_object!(<'a> &'a Droid: Database as "Droid" |&self| {
    description: "A mechanical creature in the Star Wars universe."

    interfaces: [&Character]

    field id() -> FieldResult<&str> as "The id of the droid" {
        Ok(self.id())
    }

    field name() -> FieldResult<Option<&str>> as "The name of the droid" {
        Ok(Some(self.name()))
    }

    field friends(&mut executor) -> FieldResult<Vec<&Character>>
    as "The friends of the droid" {
        Ok(executor.context().get_friends(self.as_character()))
    }

    field appears_in() -> FieldResult<&[Episode]> as "Which movies they appear in" {
        Ok(self.appears_in())
    }

    field primary_function() -> FieldResult<&Option<String>> as "The primary function of the droid" {
        Ok(self.primary_function())
    }
});


graphql_object!(Database: Database as "Query" |&self| {
    description: "The root query object of the schema"

    field human(
        id: String as "id of the human"
    ) -> FieldResult<Option<&Human>> {
        Ok(self.get_human(&id))
    }

    field droid(
        id: String as "id of the droid"
    ) -> FieldResult<Option<&Droid>> {
        Ok(self.get_droid(&id))
    }

    field hero(
        episode: Option<Episode> as
        "If omitted, returns the hero of the whole saga. If provided, returns \
        the hero of that particular episode"
    ) -> FieldResult<Option<&Character>> {
        Ok(Some(self.get_hero(episode).as_character()))
    }
});
