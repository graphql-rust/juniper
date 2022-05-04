
use crate::{graphql_scalar, InputValue, ScalarValue, Value};

use super::*;

#[graphql_scalar(with = decimal_scalar, parse_token(String))]
type Decimal = rust_decimal::Decimal;
pub mod decimal_scalar {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &Decimal) -> Value<S> {
        Value::scalar(v.to_string())
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Decimal, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                Decimal::parse_str(s).map_err(|e| format!("Failed to parse `Decimal`: {}", e))
            })
    }

    #[cfg(test)]
    mod test {
        use rust_decimal::Decimal;

        use crate::{graphql_input_value, FromInputValue, InputValue};

        #[test]
        fn uuid_from_input() {
            let raw = "4.20";
            let input: InputValue = graphql_input_value!((raw));

            let parsed: Decimal = FromInputValue::from_input_value(&input).unwrap();
            let id = Decimal::parse_str(raw).unwrap();

            assert_eq!(parsed, id);
        }
    }
}
