use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
pub struct ObjA {
    id: String,
}

#[graphql_interface(for = ObjA)]
struct Character {
    id: String,
}

fn main() {}
