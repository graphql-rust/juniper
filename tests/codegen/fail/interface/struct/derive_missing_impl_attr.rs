use juniper::{GraphQLInterface, GraphQLObject};

#[derive(GraphQLObject)]
pub struct ObjA {
    id: String,
}

#[derive(GraphQLInterface)]
#[graphql(for = ObjA)]
struct Character {
    id: String,
}

fn main() {}
