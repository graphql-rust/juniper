use juniper::{graphql_interface, GraphQLInputObject};

#[derive(GraphQLInputObject)]
pub struct ObjB {
    id: i32,
}

#[graphql_interface]
struct Character {
    id: ObjB,
}

fn main() {}
