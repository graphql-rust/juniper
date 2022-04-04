use std::sync::Arc;

struct Query;

#[juniper::graphql_object]
impl Query {
    fn ping() -> Arc<bool> {
        Arc::new(false)
    }
}

#[derive(juniper::GraphQLInputObject)]
struct Ping {
    expect_result: Arc<bool>,
}
