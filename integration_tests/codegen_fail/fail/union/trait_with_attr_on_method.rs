use juniper::{graphql_union, GraphQLObject};

#[graphql_union]
trait Character {
    #[graphql_union(with = something)]
    fn a(&self) -> Option<&Human>;
}

#[derive(GraphQLObject)]
pub struct Human {
    id: String,
}

fn main() {}
