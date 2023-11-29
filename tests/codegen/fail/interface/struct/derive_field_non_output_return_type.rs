use juniper::{GraphQLInputObject, GraphQLInterface};

#[derive(GraphQLInputObject)]
pub struct ObjB {
    id: i32,
}

#[derive(GraphQLInterface)]
struct Character {
    id: ObjB,
}

fn main() {}
