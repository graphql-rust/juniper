use juniper::{graphql_interface, GraphQLObject};

#[derive(GraphQLObject)]
pub struct ObjA {
    test: String,
}

#[graphql_interface(for = ObjA)]
impl ObjA {}

fn main() {}
