#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate juniper;

use rocket::response::content;
use rocket::State;

use juniper::tests::model::Database;
use juniper::{EmptyMutation, RootNode};

use juniper::rocket_handlers;

type Schema = RootNode<'static, Database, EmptyMutation<Database>>;

#[get("/")]
fn graphiql() -> content::Html<String> {
    rocket_handlers::graphiql_source("/graphql")
}

#[get("/graphql?<request>")]
fn get_graphql_handler(
    context: State<Database>,
    request: rocket_handlers::GraphQLRequest,
    schema: State<Schema>,
) -> rocket_handlers::GraphQLResponse {
    request.execute(&schema, &context)
}

#[post("/graphql", data="<request>")]
fn post_graphql_handler(
    context: State<Database>,
    request: rocket_handlers::GraphQLRequest,
    schema: State<Schema>,
) -> rocket_handlers::GraphQLResponse {
    request.execute(&schema, &context)
}

fn main() {
    rocket::ignite()
        .manage(Database::new())
        .manage(Schema::new(Database::new(), EmptyMutation::<Database>::new()))
        .mount("/", routes![graphiql, get_graphql_handler, post_graphql_handler])
        .launch();
}
