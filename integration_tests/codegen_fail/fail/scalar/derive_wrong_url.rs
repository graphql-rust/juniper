#[derive(juniper::GraphQLScalarValue)]
#[graphql(specified_by_url = "not an url")]
struct ScalarSpecifiedByUrl(i64);

fn main() {}
