use juniper::{graphql_scalar, InputValue, ScalarValue, Value};

#[graphql_scalar(specified_by_url = "not an url")]
struct ScalarSpecifiedByUrl(i32);

fn main() {}
