use juniper::{graphql_scalar, InputValue, ScalarValue, Value};

#[graphql_scalar(specified_by_url = "not an url", parse_token(i32))]
struct ScalarSpecifiedByUrl(i32);

impl ScalarSpecifiedByUrl {
    fn to_output<S: ScalarValue>(&self) -> Value<S> {
        Value::scalar(0)
    }

    fn from_input<S: ScalarValue>(_: &InputValue<S>) -> Result<Self, String> {
        Ok(Self)
    }
}

fn main() {}
