use juniper::{
    tests::fixtures::starwars::schema::{Database, Query},
    EmptyMutation, EmptySubscription, RootNode,
};
use rocket::{response::content::RawHtml, routes, State};

type Schema = RootNode<Query, EmptyMutation<Database>, EmptySubscription<Database>>;

#[rocket::get("/")]
async fn homepage() -> RawHtml<&'static str> {
    RawHtml(
        "<html><h1>juniper_rocket/simple example</h1>\
               <div>visit <a href=\"/graphiql\">GraphiQL</a></div>\
               <div>visit <a href=\"/playground\">GraphQL Playground</a></div>\
         </html>",
    )
}

#[rocket::get("/graphiql")]
fn graphiql() -> RawHtml<String> {
    juniper_rocket::graphiql_source("/graphql", None)
}

#[rocket::get("/playground")]
fn playground() -> RawHtml<String> {
    juniper_rocket::playground_source("/graphql", None)
}

// GET request accepts query parameters like these:
// ?query=<urlencoded-graphql-query-string>
// &operationName=<optional-name>
// &variables=<optional-json-encoded-variables>
// See details here: https://graphql.org/learn/serving-over-http#get-request
#[rocket::get("/graphql?<request..>")]
async fn get_graphql(
    db: &State<Database>,
    request: juniper_rocket::GraphQLRequest,
    schema: &State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(schema, db).await
}

#[rocket::post("/graphql", data = "<request>")]
async fn post_graphql(
    db: &State<Database>,
    request: juniper_rocket::GraphQLRequest,
    schema: &State<Schema>,
) -> juniper_rocket::GraphQLResponse {
    request.execute(schema, db).await
}

#[rocket::main]
async fn main() {
    _ = rocket::build()
        .manage(Database::new())
        .manage(Schema::new(
            Query,
            EmptyMutation::new(),
            EmptySubscription::new(),
        ))
        .mount(
            "/",
            routes![homepage, graphiql, playground, get_graphql, post_graphql],
        )
        .launch()
        .await
        .expect("server to launch");
}
