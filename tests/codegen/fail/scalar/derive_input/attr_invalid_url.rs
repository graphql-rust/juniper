use juniper::graphql_scalar;

#[graphql_scalar(specified_by_url = "not an url", transparent)]
struct ScalarSpecifiedByUrl(i32);

fn main() {}
