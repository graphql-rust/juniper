use juniper::{GraphQLInputObject, GraphQLObject};

#[derive(GraphQLObject)]
struct ObjectA {
    test: String,
}

#[derive(GraphQLInputObject)]
enum Object {
    Field(ObjectA),
}

fn main() {}
