use juniper::graphql_scalar;

#[graphql_scalar(transparent)]
struct Scalar {
    id: i32,
    another: i32,
}

fn main() {}
