use juniper::graphql_subscription;

struct Obj;

#[graphql_subscription]
impl Obj {}

fn main() {}
