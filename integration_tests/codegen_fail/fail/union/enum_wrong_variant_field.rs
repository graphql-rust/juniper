use juniper::{GraphQLObject, GraphQLUnion};

#[derive(GraphQLUnion)]
enum Character1 {
    A { human: Human },
}

#[derive(GraphQLUnion)]
enum Character2 {
    A(Human, u8),
}

#[derive(GraphQLObject)]
pub struct Human {
    id: String,
}

fn main() {}
