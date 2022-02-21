use juniper::{GraphQLInterface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    id: String,
}

type ObjAlias = ObjA;

#[derive(GraphQLInterface)]
#[graphql(for = [ObjA, ObjAlias])]
struct Character {
    id: String,
}

fn main() {}
