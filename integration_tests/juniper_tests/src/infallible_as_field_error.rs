struct Query;

#[juniper::graphql_object]
impl Query {
    fn ping() -> Result<bool, std::convert::Infallible> {
        Ok(false)
    }
}
