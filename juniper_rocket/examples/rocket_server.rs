#![feature(decl_macro, proc_macro_hygiene)]

use rocket::{response::content, State};

use juniper::{tests::model::Database, EmptyMutation, FieldError, FieldResult, RootNode, Value};

struct Query;

#[juniper::object(Context = Database)]
impl Query {
    fn users(id: i32) -> FieldResult<Vec<i32>> {
        Err(FieldError::new("users field result", Value::Null))
    }

    fn usersTwo(id: i32) -> FieldResult<Vec<i32>> {
        Err(FieldError::new("users2 field result", Value::Null))
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<Database>>;

#[rocket::get("/")]
fn graphiql() -> content::Html<String> {
    juniper_rocket::graphiql_source("/graphql")
}

#[rocket::get("/graphql?<request>")]
fn get_graphql_handler(
    context: State<Database>,
    request: juniper_rocket::GraphQLRequest,
    schema: State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&schema, &context)
}

#[rocket::post("/graphql", data = "<request>")]
fn post_graphql_handler(
    context: State<Database>,
    request: juniper_rocket::GraphQLRequest,
    schema: State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&schema, &context)
}

fn main() {
    rocket::ignite()
        .manage(Database::new())
        .manage(Schema::new(Query, EmptyMutation::<Database>::new()))
        .mount(
            "/",
            rocket::routes![graphiql, get_graphql_handler, post_graphql_handler],
        )
        .launch();
}
