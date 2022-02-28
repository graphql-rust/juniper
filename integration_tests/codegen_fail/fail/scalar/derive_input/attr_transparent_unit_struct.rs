use juniper::graphql_scalar;

#[graphql_scalar(transparent)]
struct ScalarSpecifiedByUrl;

fn main() {}
