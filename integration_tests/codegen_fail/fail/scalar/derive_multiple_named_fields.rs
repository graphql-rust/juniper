use juniper::GraphQLScalar;

#[derive(GraphQLScalar)]
struct Scalar {
    id: i32,
    another: i32,
}

fn main() {}
