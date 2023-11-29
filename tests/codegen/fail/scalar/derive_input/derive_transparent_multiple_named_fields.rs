use juniper::GraphQLScalar;

#[derive(GraphQLScalar)]
#[graphql(transparent)]
struct Scalar {
    id: i32,
    another: i32,
}

fn main() {}
