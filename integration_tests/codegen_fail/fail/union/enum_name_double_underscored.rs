use juniper::{GraphQLObject, GraphQLUnion};

#[derive(GraphQLUnion)]
enum __Character {
    A(Human),
}

#[derive(GraphQLObject)]
pub struct Human {
    id: String,
}

fn main() {}
