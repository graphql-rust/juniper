//! Ensuring that [`GraphQLResponse`] could be built by crate users.

#![expect(unused_crate_dependencies, reason = "single test case")]

use juniper_rocket::GraphQLResponse;
use rocket::http::Status;

#[test]
fn test_graphql_response_is_public() {
    let _ = GraphQLResponse(Status::Unauthorized, "Unauthorized".into());
}
