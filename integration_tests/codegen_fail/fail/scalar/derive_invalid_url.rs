use juniper::GraphQLScalarValue;

#[derive(GraphQLScalarValue)]
#[graphql(specified_by_url = "not an url")]
struct ScalarSpecifiedByUrl(i64);

fn main() {}
