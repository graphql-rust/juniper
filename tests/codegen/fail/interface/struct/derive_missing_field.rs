use juniper::{GraphQLInterface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    test: String,
}

#[derive(GraphQLInterface)]
#[graphql(for = ObjA)]
struct Character {
    id: String,
}

fn main() {}
