use juniper::graphql_scalar;

struct Scalar;

#[graphql_scalar]
type CustomScalar = Scalar;

fn main() {}
