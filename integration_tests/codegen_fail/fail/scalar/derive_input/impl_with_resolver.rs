use juniper::{graphql_scalar, ScalarValue, Value};

#[graphql_scalar(to_output_with = Scalar::to_output)]
struct Scalar;

impl Scalar {
    fn to_output<S: ScalarValue>(&self) -> Value<S> {
        Value::scalar(0)
    }
}

fn main() {}
