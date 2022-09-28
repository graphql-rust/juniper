use std::convert::Infallible;

use juniper::graphql_object;

struct Query;

#[graphql_object]
impl Query {
    fn ping() -> Result<bool, Infallible> {
        Ok(false)
    }
}
