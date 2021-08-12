use juniper::{GraphQLInputObject, GraphQLObject};

#[derive(GraphQLInputObject)]
struct ObjB {
    id: i32,
}

#[derive(GraphQLObject)]
struct ObjA {
    id: ObjB,
}

fn main() {}
