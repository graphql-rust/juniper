use juniper::{GraphQLInterface, GraphQLObject};

#[derive(GraphQLObject)]
pub struct ObjA {
    test: String,
}

#[derive(GraphQLInterface)]
#[graphql(for = ObjA)]
enum Character {}

fn main() {}
