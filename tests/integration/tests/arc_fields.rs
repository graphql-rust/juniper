#![allow(dead_code, reason = "GraphQL schema testing")]

use std::sync::Arc;

use juniper::{GraphQLInputObject, graphql_object};

struct Query;

#[graphql_object]
impl Query {
    fn ping() -> Arc<bool> {
        Arc::new(false)
    }
}

#[derive(GraphQLInputObject)]
struct Ping {
    expect_result: Arc<bool>,
}
