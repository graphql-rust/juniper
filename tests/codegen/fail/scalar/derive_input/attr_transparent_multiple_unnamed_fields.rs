use juniper::graphql_scalar;

#[graphql_scalar(transparent)]
struct Scalar(i32, i32);

fn main() {}
