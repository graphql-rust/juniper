use juniper::{graphql_union, GraphQLObject};

#[graphql_union]
trait __Character {
    fn a(&self) -> Option<&Human>;
}

#[derive(GraphQLObject)]
pub struct Human {
    id: String,
}

fn main() {}
