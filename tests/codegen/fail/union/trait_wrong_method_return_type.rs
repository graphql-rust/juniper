use juniper::{graphql_union, GraphQLObject};

#[graphql_union]
trait Character {
    fn a(&self) -> &Human;
}

#[derive(GraphQLObject)]
pub struct Human {
    id: String,
}

fn main() {}
