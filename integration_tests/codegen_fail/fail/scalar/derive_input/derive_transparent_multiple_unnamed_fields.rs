use juniper::GraphQLScalar;

#[derive(GraphQLScalar)]
#[graphql(transparent)]
struct Scalar(i32, i32);

fn main() {}
