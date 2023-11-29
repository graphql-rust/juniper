use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    id: String,
}

type ObjAlias = ObjA;

#[graphql_interface(for = [ObjA, ObjAlias])]
struct Character {
    id: String,
}

fn main() {}
