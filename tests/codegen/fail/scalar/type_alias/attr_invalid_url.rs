use juniper::{graphql_scalar, Scalar, ScalarValue, Value};

struct ScalarSpecifiedByUrl;

#[graphql_scalar]
#[graphql(
    specified_by_url = "not an url",
    with = scalar,
    parse_token(i32),
)]
type MyScalar = ScalarSpecifiedByUrl;

mod scalar {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(_: &ScalarSpecifiedByUrl) -> Value<S> {
        Value::scalar(0)
    }

    pub(super) fn from_input(_: &Scalar<impl ScalarValue>) -> ScalarSpecifiedByUrl {
        ScalarSpecifiedByUrl
    }
}

fn main() {}
