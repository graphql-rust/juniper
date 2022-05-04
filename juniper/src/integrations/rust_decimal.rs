use crate::{graphql_scalar, InputValue, ScalarValue, Value};

use std::str::FromStr;

#[graphql_scalar(with = rust_decimal_scalar, parse_token(String))]
type Decimal = rust_decimal::Decimal;
pub mod rust_decimal_scalar {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &Decimal) -> Value<S> {
        Value::scalar(v.to_string())
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Decimal, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                Decimal::from_str(s).map_err(|e| format!("Failed to parse `Decimal`: {}", e))
            })
    }

    #[cfg(test)]
    mod test {
        use super::*;

        use crate::{graphql_input_value, FromInputValue, InputValue};

        #[test]
        fn rust_decimal_from_input() {
            let raw = "4.20";
            let input: InputValue = graphql_input_value!((raw));

            let parsed: Decimal = FromInputValue::from_input_value(&input).unwrap();
            let id = Decimal::from_str(raw).unwrap();

            assert_eq!(parsed, id);
        }
    }
}
