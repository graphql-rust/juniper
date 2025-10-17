use juniper::{GraphQLInputObject, GraphQLObject};

#[derive(GraphQLObject)]
struct ObjectA {
    test: String,
}

#[derive(GraphQLInputObject)]
struct Object {
    field: ObjectA,
}

fn main() {}
