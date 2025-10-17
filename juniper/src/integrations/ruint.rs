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

use crate::graphql_scalar;

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
    with = uint_scalar,
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
    with = uint_scalar,
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
    with = uint_scalar,
    specified_by_url = "https://docs.rs/ruint",
)]
pub type U256 = ruint::aliases::U256;

pub mod uint_scalar {
    //! [GraphQL scalar][0] implementation for [`ruint::Uint`] type, suitable for specifying into
    //! the `with` argument of the `#[graphql_scalar]`][1] macro.
    //!
    //! [0]: https://spec.graphql.org/October2021#sec-Scalars
    //! [1]: macro@crate::graphql_scalar

    use crate::{ParseScalarResult, ParseScalarValue, Scalar, ScalarToken, ScalarValue};

    /// Parses an arbitrary [`ruint::Uint`] value from the provided [`ScalarValue`].
    ///
    /// Expects either `String` or `Int` GraphQL scalars as input, with standard Rust syntax for
    /// decimal, hexadecimal, binary and octal notation using prefixes `0x`, `0b` and `0o`.
    ///
    /// # Errors
    ///
    /// If the [`ruint::Uint`] value cannot be parsed from the provided [`ScalarValue`].
    pub fn from_input<const B: usize, const L: usize>(
        value: &Scalar<impl ScalarValue>,
    ) -> Result<ruint::Uint<B, L>, Box<str>> {
        if let Some(int) = value.try_to_int() {
            return ruint::Uint::try_from(int).map_err(|e| {
                format!("Failed to parse `ruint::Uint<{B}, {L}>` from `Int`: {e}").into()
            });
        }

        let Some(s) = value.try_as_str() else {
            return Err(format!(
                "Failed to parse `ruint::Uint<{B}, {L}>`: input is neither `String` nor `Int`"
            )
            .into());
        };
        s.parse().map_err(|e| {
            format!("Failed to parse `ruint::Uint<{B}, {L}>` from `String`: {e}").into()
        })
    }

    // ERGONOMICS: This method is intentionally placed here to allow omitting specifying another
    //             `to_output_with = ScalarValue::from_displayable` macro argument in the user code
    //             once the `with = juniper::integrations::ruint::uint_scalar` is specified already.
    /// Converts the provided arbitrary [`ruint::Uint`] value into a [`ScalarValue`].
    ///
    /// Always serializes as GraphQL `String` in decimal notation.
    pub fn to_output<const B: usize, const L: usize, S: ScalarValue>(int: &ruint::Uint<B, L>) -> S {
        S::from_displayable(int)
    }

    // ERGONOMICS: This method is intentionally placed here to allow omitting specifying another
    //             `parse_token(i32, String)` macro argument in the user code once the
    //             `with = juniper::integrations::ruint::uint_scalar` is specified already.
    /// Parses a [`ScalarValue`] from the provided [`ScalarToken`] as the [`ruint::Uint`] requires.
    ///
    /// # Errors
    ///
    /// If the provided [`ScalarToken`] represents neither `String` nor `Int` GraphQL scalar.
    pub fn parse_token<S: ScalarValue>(token: ScalarToken<'_>) -> ParseScalarResult<S> {
        <String as ParseScalarValue<S>>::from_str(token)
            .or_else(|_| <i32 as ParseScalarValue<S>>::from_str(token))
    }
}

#[cfg(test)]
mod test {
    use super::{U64, U128, U256};
    use crate::{FromInputValue as _, InputValue, ToInputValue as _, graphql};

    #[test]
    fn parses_correct_input_256() {
        for (input, expected) in [
            (graphql::input_value!(0), U256::ZERO),
            (graphql::input_value!(123), U256::from(123)),
            (graphql::input_value!("0"), U256::ZERO),
            (graphql::input_value!("42"), U256::from(42)),
            (graphql::input_value!("0o10"), U256::from(8)),
            (
                graphql::input_value!("0xdeadbeef"),
                U256::from(3735928559u64),
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
            (graphql::input_value!(0), U128::ZERO),
            (graphql::input_value!(123), U128::from(123)),
            (graphql::input_value!("0"), U128::ZERO),
            (graphql::input_value!("42"), U128::from(42)),
            (
                graphql::input_value!("0xdeadbeef"),
                U128::from(3735928559u64),
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
            (graphql::input_value!(0), U64::ZERO),
            (graphql::input_value!(123), U64::from(123)),
            (graphql::input_value!("0"), U64::ZERO),
            (graphql::input_value!("42"), U64::from(42)),
            (
                graphql::input_value!("0xdeadbeef"),
                U64::from(3735928559u64),
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
                parsed.unwrap(),
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
