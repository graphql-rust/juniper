//! [`Tz`] (timezone) scalar implementation, represented by its [IANA database][1] name.
//!
//! [`Tz`]: chrono_tz::Tz
//! [1]: http://www.iana.org/time-zones

use chrono_tz::Tz;

use crate::{
    graphql_scalar,
    parser::{ParseError, ScalarToken, Token},
    value::ParseScalarResult,
    GraphQLScalar, InputValue, ScalarValue, Value,
};

#[graphql_scalar(name = "Tz", description = "Timezone")]
impl<S: ScalarValue> GraphQLScalar<S> for Tz {
    type Error = String;

    fn to_output(&self) -> Value<S> {
        Value::scalar(self.name().to_owned())
    }

    fn from_input(v: &InputValue<S>) -> Result<Self, Self::Error> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                s.parse::<Tz>()
                    .map_err(|e| format!("Failed to parse `Tz`: {}", e))
            })
    }

    fn parse_token(val: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
        if let ScalarToken::String(s) = val {
            Ok(S::from(s.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(val)))
        }
    }
}

#[cfg(test)]
mod test {
    mod from_input_value {
        use std::ops::Deref;

        use chrono_tz::Tz;

        use crate::{graphql_input_value, FromInputValue, InputValue};

        fn tz_input_test(raw: &'static str, expected: Result<Tz, &str>) {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = FromInputValue::from_input_value(&input);

            assert_eq!(
                parsed.as_ref().map_err(Deref::deref),
                expected.as_ref().map_err(Deref::deref),
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
