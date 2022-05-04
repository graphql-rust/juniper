use crate::{graphql_scalar, InputValue, ScalarValue, Value};

#[graphql_scalar(with = bigdecimal_scalar, parse_token(String))]
type BigDecimal = ::bigdecimal::BigDecimal;
use std::str::FromStr;
pub mod bigdecimal_scalar {

    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &BigDecimal) -> Value<S> {
        Value::scalar(v.to_string())
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<BigDecimal, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                BigDecimal::from_str(s).map_err(|e| format!("Failed to parse `BigDecimal`: {}", e))
            })
    }

    #[cfg(test)]
    mod test {
        use super::*;

        use crate::{graphql_input_value, FromInputValue, InputValue};

        #[test]
        fn bigdecimal_from_input() {
            let raw = "4.20";
            let input: InputValue = graphql_input_value!((raw));

            let parsed: BigDecimal = FromInputValue::from_input_value(&input).unwrap();
            let id = BigDecimal::from_str(raw).unwrap();

            assert_eq!(parsed, id);
        }
    }
}
