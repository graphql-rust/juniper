use juniper::graphql_scalar;

#[graphql_scalar]
#[graphql(transparent)]
struct Scalar {
    id: i32,
    another: i32,
}

fn main() {}
