use juniper::{graphql_scalar, InputValue, ScalarValue, Value};

#[graphql_scalar(
    specified_by_url = "not an url",
    with = scalar,
    parse_token(i32),
)]
struct ScalarSpecifiedByUrl(i32);

mod scalar {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(_: &ScalarSpecifiedByUrl) -> Value<S> {
        Value::scalar(0)
    }

    pub(super) fn from_input<S: ScalarValue>(
        _: &InputValue<S>,
    ) -> Result<ScalarSpecifiedByUrl, String> {
        Ok(ScalarSpecifiedByUrl)
    }
}

fn main() {}
