//! [`apollo_compiler`] engine.
//!
//! `apollo-compiler` defines a schema with SDL and resolves fields through hand-written
//! [`ObjectValue`] implementations.

use apollo_compiler::{
    ExecutableDocument, Schema,
    resolvers::{Execution, FieldError, ObjectValue, ResolveInfo, ResolvedValue},
    response::{JsonMap, JsonValue},
    validation::Valid,
};
use serde::Deserialize;

use super::Engine;
use crate::data::{self, Episode, Role};

/// [`apollo_compiler`] engine with synchronous resolvers.
///
/// Each request parses and validates the document, and executes it with
/// [`Execution::execute_sync()`].
pub struct ApolloCompiler {
    schema: Valid<Schema>,
}

impl Default for ApolloCompiler {
    fn default() -> Self {
        Self {
            schema: Schema::parse_and_validate(include_str!("schema.graphql"), "schema.graphql")
                .expect("valid schema"),
        }
    }
}

impl Engine for ApolloCompiler {
    const NAME: &'static str = "apollo_compiler";
    const SUPPORTS_IS_ONE_OF: bool = false;

    fn execute(&self, request: &str) -> String {
        let request: Request = serde_json::from_str(request).expect("valid request");
        let document = match ExecutableDocument::parse_and_validate(
            &self.schema,
            request.query,
            "request.graphql",
        ) {
            Ok(document) => document,
            Err(invalid) => return error_response(invalid.errors),
        };
        let response = Execution::new(&self.schema, &document)
            .operation_name(request.operation_name.as_deref())
            .and_then(|execution| {
                execution
                    .raw_variable_values(&request.variables)
                    .enable_schema_introspection(true)
                    .execute_sync(&Query)
            });
        match response {
            Ok(response) => serde_json::to_string(&response).expect("response serializes to JSON"),
            Err(error) => error_response(error.message()),
        }
    }
}

/// GraphQL-over-HTTP request body.
#[derive(Deserialize)]
struct Request {
    query: String,

    #[serde(rename = "operationName")]
    operation_name: Option<String>,

    #[serde(default)]
    variables: JsonMap,
}

/// Returns the JSON body of a response with the `error` message.
fn error_response(error: impl std::fmt::Display) -> String {
    serde_json::json!({ "errors": [{ "message": error.to_string() }] }).to_string()
}

/// Root query type.
struct Query;

impl ObjectValue for Query {
    fn type_name(&self) -> &str {
        "Query"
    }

    fn resolve_field<'a>(
        &'a self,
        info: &'a ResolveInfo<'a>,
    ) -> Result<ResolvedValue<'a>, FieldError> {
        match info.field_name() {
            "hello" => Ok(ResolvedValue::leaf("world")),
            "users" => Ok(ResolvedValue::list(
                data::first_users(int_argument(info, "first"))
                    .map(|user| ResolvedValue::object(User(user))),
            )),
            "hero" => {
                let episode = info
                    .arguments()
                    .get("episode")
                    .and_then(JsonValue::as_str)
                    .map(episode_from_name);
                Ok(character(data::hero(episode)))
            }
            "humans" => Ok(ResolvedValue::list(
                data::HUMANS
                    .into_iter()
                    .map(|human| ResolvedValue::object(Human(human))),
            )),
            "droids" => Ok(ResolvedValue::list(
                data::DROIDS
                    .into_iter()
                    .map(|droid| ResolvedValue::object(Droid(droid))),
            )),
            _ => Err(self.unknown_field_error(info)),
        }
    }
}

struct User(&'static data::User);

impl ObjectValue for User {
    fn type_name(&self) -> &str {
        "User"
    }

    fn resolve_field<'a>(
        &'a self,
        info: &'a ResolveInfo<'a>,
    ) -> Result<ResolvedValue<'a>, FieldError> {
        match info.field_name() {
            "id" => Ok(ResolvedValue::leaf(self.0.id)),
            "name" => Ok(ResolvedValue::leaf(self.0.name.as_str())),
            "email" => Ok(ResolvedValue::leaf(self.0.email.as_str())),
            "active" => Ok(ResolvedValue::leaf(self.0.active)),
            "score" => Ok(ResolvedValue::leaf(self.0.score)),
            "role" => Ok(ResolvedValue::leaf(role_name(self.0.role))),
            "friends" => Ok(ResolvedValue::list(
                self.0
                    .friends(int_argument(info, "first"))
                    .map(|user| ResolvedValue::object(User(user))),
            )),
            _ => Err(self.unknown_field_error(info)),
        }
    }
}

struct Human(&'static data::Human);

impl ObjectValue for Human {
    fn type_name(&self) -> &str {
        "Human"
    }

    fn resolve_field<'a>(
        &'a self,
        info: &'a ResolveInfo<'a>,
    ) -> Result<ResolvedValue<'a>, FieldError> {
        match info.field_name() {
            "id" => Ok(ResolvedValue::leaf(self.0.id)),
            "name" => Ok(ResolvedValue::leaf(self.0.name)),
            "friends" => Ok(ResolvedValue::list(
                self.0.friends.iter().copied().map(character),
            )),
            "appearsIn" => Ok(episodes(self.0.appears_in)),
            "homePlanet" => Ok(ResolvedValue::leaf(
                self.0.home_planet.map_or(JsonValue::Null, JsonValue::from),
            )),
            _ => Err(self.unknown_field_error(info)),
        }
    }
}

struct Droid(&'static data::Droid);

impl ObjectValue for Droid {
    fn type_name(&self) -> &str {
        "Droid"
    }

    fn resolve_field<'a>(
        &'a self,
        info: &'a ResolveInfo<'a>,
    ) -> Result<ResolvedValue<'a>, FieldError> {
        match info.field_name() {
            "id" => Ok(ResolvedValue::leaf(self.0.id)),
            "name" => Ok(ResolvedValue::leaf(self.0.name)),
            "friends" => Ok(ResolvedValue::list(
                self.0.friends.iter().copied().map(character),
            )),
            "appearsIn" => Ok(episodes(self.0.appears_in)),
            "primaryFunction" => Ok(ResolvedValue::leaf(self.0.primary_function)),
            _ => Err(self.unknown_field_error(info)),
        }
    }
}

/// Returns the value of a `character` for a field of the `Character` interface type.
fn character<'a>(character: data::Character) -> ResolvedValue<'a> {
    match character {
        data::Character::Human(human) => ResolvedValue::object(Human(human)),
        data::Character::Droid(droid) => ResolvedValue::object(Droid(droid)),
    }
}

/// Returns a list value of the GraphQL names of the `episodes`.
fn episodes<'a>(episodes: &'static [Episode]) -> ResolvedValue<'a> {
    ResolvedValue::list(
        episodes
            .iter()
            .map(|episode| ResolvedValue::leaf(episode_name(*episode))),
    )
}

/// Returns the `Int!` argument with the `name`. Validation guarantees its presence.
fn int_argument(info: &ResolveInfo<'_>, name: &str) -> i32 {
    info.arguments()
        .get(name)
        .and_then(JsonValue::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .expect("validated `Int!` argument")
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::Admin => "ADMIN",
        Role::Moderator => "MODERATOR",
        Role::Member => "MEMBER",
        Role::Guest => "GUEST",
    }
}

fn episode_name(episode: Episode) -> &'static str {
    match episode {
        Episode::NewHope => "NEW_HOPE",
        Episode::Empire => "EMPIRE",
        Episode::Jedi => "JEDI",
    }
}

fn episode_from_name(name: &str) -> Episode {
    match name {
        "NEW_HOPE" => Episode::NewHope,
        "EMPIRE" => Episode::Empire,
        "JEDI" => Episode::Jedi,
        _ => unreachable!("validation rejects unknown `Episode` values"),
    }
}
