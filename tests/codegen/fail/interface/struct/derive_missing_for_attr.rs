use juniper::{GraphQLInterface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    id: String,
}

#[derive(GraphQLInterface)]
struct Character {
    id: String,
}

fn main() {}
