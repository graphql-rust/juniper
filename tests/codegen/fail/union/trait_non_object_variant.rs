use juniper::{graphql_union, GraphQLEnum};

#[derive(GraphQLEnum)]
pub enum Test {
    A,
    B,
}

#[graphql_union]
trait Character {
    fn a(&self) -> Option<&Test>;
}

fn main() {}
