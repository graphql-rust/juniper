#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate juniper;
extern crate juniper_rocket;
extern crate rocket;

use std::io::Cursor;

use rocket::http::ContentType;
use rocket::local::{Client, LocalRequest};
use rocket::Response;
use rocket::Rocket;
use rocket::State;

use juniper::http::tests as http_tests;
use juniper::tests::model::Database;
use juniper::EmptyMutation;
use juniper::RootNode;

use juniper_rocket::GraphQLRequest;
use juniper_rocket::GraphQLResponse;

type Schema = RootNode<'static, Database, EmptyMutation<Database>>;

#[get("/?<request>")]
fn get_graphql_handler<'a>(
    context: State<Database>,
    request: GraphQLRequest,
    schema: State<Schema>,
) -> Response<'a> {
    let GraphQLResponse(status, json) = request.execute(&schema, &context);
    Response::build()
        .raw_header("X-Custom-Rocket-Response", "It works!")
        .header(ContentType::new("application", "json"))
        .status(status)
        .sized_body(Cursor::new(json))
        .finalize()
}

#[post("/", data = "<request>")]
fn post_graphql_handler<'a>(
    context: State<Database>,
    request: GraphQLRequest,
    schema: State<Schema>,
) -> Response<'a> {
    let GraphQLResponse(status, json) = request.execute(&schema, &context);
    Response::build()
        .raw_header("X-Custom-Rocket-Response", "It works!")
        .header(ContentType::new("application", "json"))
        .status(status)
        .sized_body(Cursor::new(json))
        .finalize()
}

struct TestRocketIntegration {
    client: Client,
}

impl http_tests::HTTPIntegration for TestRocketIntegration {
    fn get(&self, url: &str) -> http_tests::TestResponse {
        let req = &self.client.get(url);
        make_test_response(req)
    }

    fn post(&self, url: &str, body: &str) -> http_tests::TestResponse {
        let req = &self.client.post(url).header(ContentType::JSON).body(body);
        make_test_response(req)
    }
}

#[test]
fn test_rocket_integration() {
    let rocket = make_rocket();
    let client = Client::new(rocket).expect("valid rocket");
    let integration = TestRocketIntegration { client };

    http_tests::run_http_test_suite(&integration);
}

fn make_rocket() -> Rocket {
    rocket::ignite()
        .manage(Database::new())
        .manage(Schema::new(
            Database::new(),
            EmptyMutation::<Database>::new(),
        )).mount("/", routes![post_graphql_handler, get_graphql_handler])
}

fn make_test_response<'r>(request: &LocalRequest<'r>) -> http_tests::TestResponse {
    let mut response = request.cloned_dispatch();
    let status_code = response.status().code as i32;
    let content_type = response
        .content_type()
        .expect("No content type header from handler")
        .to_string();
    let body = response
        .body()
        .expect("No body returned from GraphQL handler")
        .into_string();

    http_tests::TestResponse {
        status_code: status_code,
        body: body,
        content_type: content_type,
    }
}
