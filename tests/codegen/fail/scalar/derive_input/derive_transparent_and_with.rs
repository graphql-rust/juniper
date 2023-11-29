use juniper::GraphQLScalar;

#[derive(GraphQLScalar)]
#[graphql(with = Self, transparent)]
struct Scalar;

fn main() {}
