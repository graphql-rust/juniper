//! [`Tz`] (timezone) scalar implementation, represented by its [IANA database][1] name.
//!
//! [`Tz`]: chrono_tz::Tz
//! [1]: http://www.iana.org/time-zones

use crate::{graphql_scalar, InputValue, ScalarValue, Value};

#[graphql_scalar(with = tz, parse_token(String))]
type Tz = chrono_tz::Tz;

mod tz {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &Tz) -> Value<S> {
        Value::scalar(v.name().to_owned())
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Tz, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                s.parse::<Tz>()
                    .map_err(|e| format!("Failed to parse `Tz`: {}", e))
            })
    }
}

#[cfg(test)]
mod test {
    mod from_input {
        use chrono_tz::Tz;

        use crate::{graphql_input_value, FromInputValue, InputValue, IntoFieldError};

        fn tz_input_test(raw: &'static str, expected: Result<Tz, &str>) {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = FromInputValue::from_input_value(&input);

            assert_eq!(
                parsed.as_ref(),
                expected.map_err(IntoFieldError::into_field_error).as_ref(),
            );
        }

        #[test]
        fn europe_zone() {
            tz_input_test("Europe/London", Ok(chrono_tz::Europe::London));
        }

        #[test]
        fn etc_minus() {
            tz_input_test("Etc/GMT-3", Ok(chrono_tz::Etc::GMTMinus3));
        }

        mod invalid {
            use super::tz_input_test;

            #[test]
            fn forward_slash() {
                tz_input_test(
                    "Abc/Xyz",
                    Err("Failed to parse `Tz`: received invalid timezone"),
                );
            }

            #[test]
            fn number() {
                tz_input_test(
                    "8086",
                    Err("Failed to parse `Tz`: received invalid timezone"),
                );
            }

            #[test]
            fn no_forward_slash() {
                tz_input_test(
                    "AbcXyz",
                    Err("Failed to parse `Tz`: received invalid timezone"),
                );
            }
        }
    }
}
