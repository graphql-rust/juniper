//! GraphQL support for [`bigdecimal`] crate types.
//!
//! # Supported types
//!
//! | Rust type      | GraphQL scalar |
//! |----------------|----------------|
//! | [`BigDecimal`] | `BigDecimal`   |
//!
//! [`BigDecimal`]: bigdecimal::BigDecimal

use std::str::FromStr as _;

use crate::{Raw, ScalarValue, Value, graphql_scalar};

// TODO: Try remove on upgrade of `bigdecimal` crate.
mod for_minimal_versions_check_only {
    use num_bigint as _;
}

/// Big decimal type.
///
/// Allows storing any real number to arbitrary precision; which avoids common
/// floating point errors (such as 0.1 + 0.2 ≠ 0.3) at the cost of complexity.
///
/// Always serializes as `String`. But may be deserialized from `Int` and
/// `Float` values too. It's not recommended to deserialize from a `Float`
/// directly, as the floating point representation may be unexpected.
///
/// See also [`bigdecimal`] crate for details.
///
/// [`bigdecimal`]: https://docs.rs/bigdecimal
#[graphql_scalar(
    with = bigdecimal_scalar,
    parse_token(i32, f64, String),
    specified_by_url = "https://docs.rs/bigdecimal",
)]
type BigDecimal = bigdecimal::BigDecimal;

mod bigdecimal_scalar {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &BigDecimal) -> Value<S> {
        Value::scalar(v.to_string())
    }

    pub(super) fn from_input(v: &Raw<impl ScalarValue>) -> Result<BigDecimal, Box<str>> {
        if let Some(i) = v.try_to_int() {
            Ok(BigDecimal::from(i))
        } else if let Some(f) = v.try_to_float() {
            // See akubera/bigdecimal-rs#103 for details:
            // https://github.com/akubera/bigdecimal-rs/issues/103
            let mut buf = ryu::Buffer::new();
            BigDecimal::from_str(buf.format(f))
                .map_err(|e| format!("Failed to parse `BigDecimal` from `Float`: {e}").into())
        } else {
            v.try_to::<&str>()
                .map_err(|e| e.to_string().into())
                .and_then(|s| {
                    BigDecimal::from_str(s).map_err(|e| {
                        format!("Failed to parse `BigDecimal` from `String`: {e}").into()
                    })
                })
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr as _;

    use crate::{FromInputValue as _, InputValue, ToInputValue as _, graphql_input_value};

    use super::BigDecimal;

    #[test]
    fn parses_correct_input() {
        for (input, expected) in [
            (graphql_input_value!("4.20"), "4.20"),
            (graphql_input_value!("0"), "0"),
            (
                graphql_input_value!("999999999999.999999999"),
                "999999999999.999999999",
            ),
            (
                graphql_input_value!("87553378877997984345"),
                "87553378877997984345",
            ),
            (graphql_input_value!(123), "123"),
            (graphql_input_value!(0), "0"),
            (graphql_input_value!(43.44), "43.44"),
        ] {
            let input: InputValue = input;
            let parsed = BigDecimal::from_input_value(&input);
            let expected = BigDecimal::from_str(expected).unwrap();

            assert!(
                parsed.is_ok(),
                "failed to parse `{input:?}`: {:?}",
                parsed.unwrap_err(),
            );
            assert_eq!(parsed.unwrap(), expected, "input: {input:?}");
        }
    }

    #[test]
    fn fails_on_invalid_input() {
        for input in [
            graphql_input_value!(""),
            graphql_input_value!("0,0"),
            graphql_input_value!("12,"),
            graphql_input_value!("1996-12-19T14:23:43"),
            graphql_input_value!("i'm not even a number"),
            graphql_input_value!(null),
            graphql_input_value!(false),
        ] {
            let input: InputValue = input;
            let parsed = BigDecimal::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for raw in [
            "4.20",
            "0",
            "999999999999.999999999",
            "87553378877997984345",
            "123",
            "43.44",
        ] {
            let actual: InputValue = BigDecimal::from_str(raw).unwrap().to_input_value();

            assert_eq!(actual, graphql_input_value!((raw)), "on value: {raw}");
        }
    }
}
