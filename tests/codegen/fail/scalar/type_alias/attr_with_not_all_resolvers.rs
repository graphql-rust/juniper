use juniper::{graphql_scalar, Value};

struct Scalar;

#[graphql_scalar]
#[graphql(to_output_with = Scalar::to_output)]
type CustomScalar = Scalar;

impl Scalar {
    fn to_output(&self) -> Value {
        Value::scalar(0)
    }
}

fn main() {}
