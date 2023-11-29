//! GraphQL support for [`rust_decimal`] crate types.
//!
//! # Supported types
//!
//! | Rust type   | GraphQL scalar |
//! |-------------|----------------|
//! | [`Decimal`] | `Decimal`      |
//!
//! [`Decimal`]: rust_decimal::Decimal

use std::str::FromStr as _;

use crate::{graphql_scalar, InputValue, ScalarValue, Value};

/// 128 bit representation of a fixed-precision decimal number.
///
/// The finite set of values of `Decimal` scalar are of the form
/// m / 10<sup>e</sup>, where m is an integer such that
/// -2<sup>96</sup> < m < 2<sup>96</sup>, and e is an integer between 0 and 28
/// inclusive.
///
/// Always serializes as `String`. But may be deserialized from `Int` and
/// `Float` values too. It's not recommended to deserialize from a `Float`
/// directly, as the floating point representation may be unexpected.
///
/// See also [`rust_decimal`] crate for details.
///
/// [`rust_decimal`]: https://docs.rs/rust_decimal
#[graphql_scalar(
    with = rust_decimal_scalar,
    parse_token(i32, f64, String),
    specified_by_url = "https://docs.rs/rust_decimal",
)]
type Decimal = rust_decimal::Decimal;

mod rust_decimal_scalar {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &Decimal) -> Value<S> {
        Value::scalar(v.to_string())
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Decimal, String> {
        if let Some(i) = v.as_int_value() {
            Ok(Decimal::from(i))
        } else if let Some(f) = v.as_float_value() {
            Decimal::try_from(f).map_err(|e| format!("Failed to parse `Decimal` from `Float`: {e}"))
        } else {
            v.as_string_value()
                .ok_or_else(|| format!("Expected `String`, found: {v}"))
                .and_then(|s| {
                    Decimal::from_str(s)
                        .map_err(|e| format!("Failed to parse `Decimal` from `String`: {e}"))
                })
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr as _;

    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::Decimal;

    #[test]
    fn parses_correct_input() {
        for (input, expected) in [
            (graphql_input_value!("4.20"), "4.20"),
            (graphql_input_value!("0"), "0"),
            (graphql_input_value!("999.999999999"), "999.999999999"),
            (graphql_input_value!("875533788"), "875533788"),
            (graphql_input_value!(123), "123"),
            (graphql_input_value!(0), "0"),
            (graphql_input_value!(43.44), "43.44"),
        ] {
            let input: InputValue = input;
            let parsed = Decimal::from_input_value(&input);
            let expected = Decimal::from_str(expected).unwrap();

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
            graphql_input_value!("99999999999999999999999999999999999999"),
            graphql_input_value!("99999999999999999999999999999999999999.99"),
            graphql_input_value!("i'm not even a number"),
            graphql_input_value!(null),
            graphql_input_value!(false),
        ] {
            let input: InputValue = input;
            let parsed = Decimal::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for raw in ["4.20", "0", "999.999999999", "875533788", "123", "43.44"] {
            let actual: InputValue = Decimal::from_str(raw).unwrap().to_input_value();

            assert_eq!(actual, graphql_input_value!((raw)), "on value: {raw}");
        }
    }
}
