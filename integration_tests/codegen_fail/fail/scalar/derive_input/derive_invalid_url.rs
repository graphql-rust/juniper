use juniper::GraphQLScalar;

#[derive(GraphQLScalar)]
#[graphql(specified_by_url = "not an url", transparent)]
struct ScalarSpecifiedByUrl(i64);

fn main() {}
