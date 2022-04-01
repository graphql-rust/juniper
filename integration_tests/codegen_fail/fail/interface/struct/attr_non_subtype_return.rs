use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
pub struct ObjA {
    id: Vec<String>,
}

#[graphql_interface(for = ObjA)]
struct Character {
    id: String,
}

fn main() {}
