use juniper::{GraphQLInterface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    id: String,
}

#[derive(GraphQLInterface)]
#[graphql(for = [ObjA, ObjA])]
struct Character {
    id: String,
}

fn main() {}
