use juniper::{graphql_union, GraphQLObject};

#[graphql_union]
trait Character {
    fn a(&self, ctx: &(), rand: u8) -> Option<&Human>;
}

#[derive(GraphQLObject)]
pub struct Human {
    id: String,
}

fn main() {}
