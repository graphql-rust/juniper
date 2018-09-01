extern crate juniper_rocket;
extern crate rocket;

use rocket::http::Status;

use juniper_rocket::GraphQLResponse;

#[test]
fn test_graphql_response_is_public() {
    let _ = GraphQLResponse(Status::Unauthorized, "Unauthorized".to_string());
}
