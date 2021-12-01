//! [`Tz`] (timezone) scalar implementation, represented by its [IANA database][1] name.
//!
//! [`Tz`]: chrono_tz::Tz
//! [1]: http://www.iana.org/time-zones

use chrono_tz::Tz;

use crate::{
    graphql_scalar,
    parser::{ParseError, ScalarToken, Token},
    value::ParseScalarResult,
    FieldError, Value,
};

#[graphql_scalar(name = "Tz", description = "Timezone")]
impl<S> GraphQLScalar for Tz
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.name().to_owned())
    }

    fn from_input_value(v: &InputValue) -> Result<Tz, FieldError> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected String, found: {}", v))
            .and_then(|s| {
                s.parse::<Tz>()
                    .map_err(|e| format!("Failed to parse Timezone: {}", e))
            })
            .map_err(Into::into)
    }

    fn from_str<'a>(val: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
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
        use chrono_tz::Tz;

        use crate::{graphql_input_value, FieldError, FromInputValue, InputValue};

        fn tz_input_test(raw: &'static str, expected: Result<Tz, &str>) {
            let input: InputValue = graphql_input_value!((raw));
            let parsed =
                FromInputValue::from_input_value(&input).map_err(|e: FieldError| e.message);

            assert_eq!(parsed, expected.map_err(str::to_owned));
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
                    Err("Failed to parse Timezone: received invalid timezone"),
                );
            }

            #[test]
            fn number() {
                tz_input_test(
                    "8086",
                    Err("Failed to parse Timezone: received invalid timezone"),
                );
            }

            #[test]
            fn no_forward_slash() {
                tz_input_test(
                    "AbcXyz",
                    Err("Failed to parse Timezone: received invalid timezone"),
                );
            }
        }
    }
}
