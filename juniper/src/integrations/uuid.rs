//! GraphQL support for [uuid](https://doc.rust-lang.org/uuid/uuid/struct.Uuid.html) types.

#![allow(clippy::needless_lifetimes)]

use crate::{graphql_scalar, InputValue, ScalarValue, Value};

#[graphql_scalar(with = uuid_scalar, parse_token(String))]
type Uuid = uuid::Uuid;

mod uuid_scalar {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &Uuid) -> Value<S> {
        Value::scalar(v.to_string())
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Uuid, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| Uuid::parse_str(s).map_err(|e| format!("Failed to parse `Uuid`: {e}")))
    }
}

#[cfg(test)]
mod test {
    use uuid::Uuid;

    use crate::{graphql_input_value, FromInputValue, InputValue};

    #[test]
    fn uuid_from_input() {
        let raw = "123e4567-e89b-12d3-a456-426655440000";
        let input: InputValue = graphql_input_value!((raw));

        let parsed: Uuid = FromInputValue::from_input_value(&input).unwrap();
        let id = Uuid::parse_str(raw).unwrap();

        assert_eq!(parsed, id);
    }
}
