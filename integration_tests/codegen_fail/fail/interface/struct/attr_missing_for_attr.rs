use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    id: String,
}

#[graphql_interface]
struct Character {
    id: String,
}

fn main() {}
