//! GraphQL support for [uuid](https://doc.rust-lang.org/uuid/uuid/struct.Uuid.html) types.

#![allow(clippy::needless_lifetimes)]

use crate::{graphql_scalar, InputValue, ScalarValue, Value};

/// [Universally Unique Identifier][0] (UUID).
///
/// [`UUID` scalar][1] compliant.
///
/// See also [`uuid::Uuid`][2] for details.
///
/// [0]: https://en.wikipedia.org/wiki/Universally_unique_identifier
/// [1]: https://graphql-scalars.dev/docs/scalars/uuid
/// [2]: https://docs.rs/uuid/*/uuid/struct.Uuid.html
#[graphql_scalar(
    name = "UUID",
    with = uuid_scalar,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/uuid",
)]
type Uuid = uuid::Uuid;

mod uuid_scalar {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &Uuid) -> Value<S> {
        Value::scalar(v.to_string())
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Uuid, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| Uuid::parse_str(s).map_err(|e| format!("Failed to parse `UUID`: {e}")))
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
