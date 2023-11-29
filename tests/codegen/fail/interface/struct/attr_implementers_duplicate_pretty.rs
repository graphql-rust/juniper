use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    id: String,
}

#[graphql_interface(for = [ObjA, ObjA])]
struct Character {
    id: String,
}

fn main() {}
