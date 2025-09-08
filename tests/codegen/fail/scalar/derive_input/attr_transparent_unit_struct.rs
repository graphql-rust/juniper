use juniper::graphql_scalar;

#[graphql_scalar]
#[graphql(transparent)]
struct ScalarSpecifiedByUrl;

fn main() {}
