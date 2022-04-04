use juniper::{graphql_union, GraphQLObject};

#[graphql_union(on Human = some_fn)]
trait Character {
    fn a(&self) -> Option<&Human>;
}

#[derive(GraphQLObject)]
pub struct Human {
    id: String,
}

fn main() {}
