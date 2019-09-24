// This file is used for testing implementantions
//_!! All changes should be reset before merging to master !!__

#![feature(decl_macro, proc_macro_hygiene)]

use rocket::{response::content, State};

use juniper::{RootNode, FieldResult};

#[derive(juniper::GraphQLObject)]
#[graphql(description = "A humanoid creature in the Star Wars universe")]
struct Human {
    id: String,
    name: String,
    home_planet: String,
}

struct MyMutation;

#[juniper::object]
impl MyMutation {
    fn human(id: String) -> FieldResult<Human> {
        let human = Human {
            id: "123".to_string(),
            name: "Human Name".to_string(),
            home_planet: "Human Home Planet".to_string(),
        };
        Ok(human)
    }
}

struct MyQuery;

#[juniper::object]
impl MyQuery {
    fn human(id: String) -> FieldResult<Human> {
        let human = Human {
            id: "000".to_string(),
            name: "Query Human".to_string(),
            home_planet: "Query Human Planet".to_string(),
        };
        Ok(human)
    }
}

type Schema = RootNode<'static, MyQuery, MyMutation, MyMutation>;

#[rocket::get("/")]
fn graphiql() -> content::Html<String> {
    juniper_rocket::graphiql_source("/graphql")
}

#[rocket::get("/graphql?<request>")]
fn get_graphql_handler(
    request: juniper_rocket::GraphQLRequest,
    schema: State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&schema, &())
}

#[rocket::post("/graphql", data = "<request>")]
fn post_graphql_handler(
    request: juniper_rocket::GraphQLRequest,
    schema: State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&schema, &())
}

fn main() {
    rocket::ignite()
        .manage(Schema::new(MyQuery, MyMutation, MyMutation))
        .mount(
            "/",
            rocket::routes![graphiql, get_graphql_handler, post_graphql_handler],
        )
        .launch();
}
