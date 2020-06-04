use juniper::{GraphQLObject, GraphQLUnion};

#[derive(GraphQLUnion)]
#[graphql(on Human = resolve_fn1)]
enum Character {
    #[graphql(with = resolve_fn2)]
    A(Human),
}

#[derive(GraphQLObject)]
pub struct Human {
    id: String,
}

fn main() {}
