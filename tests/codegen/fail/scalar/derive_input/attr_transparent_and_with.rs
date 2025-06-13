use juniper::graphql_scalar;

#[graphql_scalar]
#[graphql(with = Self, transparent)]
struct Scalar;

fn main() {}
