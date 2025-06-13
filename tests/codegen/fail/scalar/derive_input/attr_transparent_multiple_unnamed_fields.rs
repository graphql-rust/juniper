use juniper::graphql_scalar;

#[graphql_scalar]
#[graphql(transparent)]
struct Scalar(i32, i32);

fn main() {}
