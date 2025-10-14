//! GraphQL support for [`ruint`] crate types.
//!
//! # Supported types
//!
//! | Rust type      | GraphQL scalar |
//! |----------------|----------------|
//! | [`U256`]       | `U256`         |
//! | [`U128`]       | `U128`         |
//! | [`U64`]        | `U64`          |
//!
//! [`U256`]: ruint::aliases::U256
//! [`U128`]: ruint::aliases::U128
//! [`U64`]: ruint::aliases::U64

use crate::{ScalarValue, graphql_scalar};

/// Uint type using const generics.
///
/// Always serializes as `String` in decimal notation.
/// May be deserialized from `i32` and `String` with
/// standard Rust syntax for decimal, hexadecimal, binary and octal
/// notation using prefixes 0x, 0b and 0o.
///
/// Confusingly empty strings get parsed as 0
/// https://github.com/recmo/uint/issues/348
#[graphql_scalar]
#[graphql(
    with = ruint_scalar,
    to_output_with = ScalarValue::from_displayable,
    parse_token(i32, String),
    specified_by_url = "https://docs.rs/ruint",
)]
pub type U64 = ruint::aliases::U64;

/// Uint type using const generics.
///
/// Always serializes as `String` in decimal notation.
/// May be deserialized from `i32` and `String` with
/// standard Rust syntax for decimal, hexadecimal, binary and octal
/// notation using prefixes 0x, 0b and 0o.
///
/// Confusingly empty strings get parsed as 0
/// https://github.com/recmo/uint/issues/348
#[graphql_scalar]
#[graphql(
    with = ruint_scalar,
    to_output_with = ScalarValue::from_displayable,
    parse_token(i32, String),
    specified_by_url = "https://docs.rs/ruint",
)]
pub type U128 = ruint::aliases::U128;

/// Uint type using const generics.
///
/// Always serializes as `String` in decimal notation.
/// May be deserialized from `i32` and `String` with
/// standard Rust syntax for decimal, hexadecimal, binary and octal
/// notation using prefixes 0x, 0b and 0o.
///
/// Confusingly empty strings get parsed as 0
/// https://github.com/recmo/uint/issues/348
#[graphql_scalar]
#[graphql(
    with = ruint_scalar,
    to_output_with = ScalarValue::from_displayable,
    parse_token(i32, String),
    specified_by_url = "https://docs.rs/ruint",
)]
pub type U256 = ruint::aliases::U256;

mod ruint_scalar {
    use std::str::FromStr;

    use crate::{Scalar, ScalarValue};

    pub(super) fn from_input<const B: usize, const L: usize>(
        v: &Scalar<impl ScalarValue>,
    ) -> Result<ruint::Uint<B, L>, Box<str>> {
        if let Some(int) = v.try_to_int() {
            return ruint::Uint::try_from(int)
                .map_err(|e| format!("Failt to parse `Uint<{B},{L}>`: {e}").into());
        }

        let Some(str) = v.try_as_str() else {
            return Err(
                format!("Failt to parse `Uint<{B},{L}>`: input is not `String` or `Int`").into(),
            );
        };

        ruint::Uint::from_str(str)
            .map_err(|e| format!("Failt to parse `Uint<{B},{L}>`: {e}").into())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        FromInputValue as _, InputValue, ToInputValue as _, graphql,
        integrations::ruint::{U64, U128, U256},
    };

    #[test]
    fn parses_correct_input_256() {
        for (input, expected) in [
            (graphql::input_value!(0), ruint::aliases::U256::ZERO),
            (graphql::input_value!(123), ruint::aliases::U256::from(123)),
            (graphql::input_value!("0"), ruint::aliases::U256::ZERO),
            (graphql::input_value!("42"), ruint::aliases::U256::from(42)),
            (graphql::input_value!("0o10"), ruint::aliases::U256::from(8)),
            (
                graphql::input_value!("0xdeadbeef"),
                ruint::aliases::U256::from(3735928559u64),
            ),
        ] {
            let input: InputValue = input;
            let parsed = U256::from_input_value(&input);

            assert!(
                parsed.is_ok(),
                "failed to parse `{input:?}`: {:?}",
                parsed.unwrap_err(),
            );

            assert_eq!(parsed.unwrap(), expected, "input: {input:?}");
        }
    }

    #[test]
    fn parses_correct_input_128() {
        for (input, expected) in [
            (graphql::input_value!(0), ruint::aliases::U128::ZERO),
            (graphql::input_value!(123), ruint::aliases::U128::from(123)),
            (graphql::input_value!("0"), ruint::aliases::U128::ZERO),
            (graphql::input_value!("42"), ruint::aliases::U128::from(42)),
            (
                graphql::input_value!("0xdeadbeef"),
                ruint::aliases::U128::from(3735928559u64),
            ),
        ] {
            let input: InputValue = input;
            let parsed = U128::from_input_value(&input);

            assert!(
                parsed.is_ok(),
                "failed to parse `{input:?}`: {:?}",
                parsed.unwrap_err(),
            );

            assert_eq!(parsed.unwrap(), expected, "input: {input:?}");
        }
    }

    #[test]
    fn parses_correct_input_64() {
        for (input, expected) in [
            (graphql::input_value!(0), ruint::aliases::U64::ZERO),
            (graphql::input_value!(123), ruint::aliases::U64::from(123)),
            (graphql::input_value!("0"), ruint::aliases::U64::ZERO),
            (graphql::input_value!("42"), ruint::aliases::U64::from(42)),
            (
                graphql::input_value!("0xdeadbeef"),
                ruint::aliases::U64::from(3735928559u64),
            ),
        ] {
            let input: InputValue = input;
            let parsed = U64::from_input_value(&input);

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
            graphql::input_value!("0,0"),
            graphql::input_value!("12,"),
            graphql::input_value!("1996-12-19T14:23:43"),
            graphql::input_value!("i'm not even a number"),
            graphql::input_value!(null),
            graphql::input_value!(false),
            graphql::input_value!(-123),
        ] {
            let input: InputValue = input;
            let parsed = U256::from_input_value(&input);

            assert!(
                parsed.is_err(),
                "allows input: {input:?} {}",
                parsed.unwrap()
            );
        }
    }

    #[test]
    fn formats_correctly() {
        for (raw, expected) in [
            ("0", "0"),
            ("87553378877997984345", "87553378877997984345"),
            ("123", "123"),
            ("0x42", "66"),
        ] {
            let actual: InputValue = raw.parse::<U256>().unwrap().to_input_value();

            assert_eq!(actual, graphql::input_value!((expected)), "on value: {raw}");
        }
    }
}
