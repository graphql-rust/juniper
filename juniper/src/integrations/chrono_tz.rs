//! [`Tz`] (timezone) scalar implementation, represented by its [IANA database][1] name.
//!
//! [`Tz`]: chrono_tz::Tz
//! [1]: http://www.iana.org/time-zones

use chrono_tz::Tz;

use crate::{
    graphql_scalar,
    parser::{ParseError, ScalarToken, Token},
    value::ParseScalarResult,
    Value,
};

#[graphql_scalar(name = "Tz", description = "Timezone")]
impl<S> GraphQLScalar for Tz
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.name().to_owned())
    }

    fn from_input_value(v: &InputValue) -> Option<Tz> {
        v.as_string_value().and_then(|s| s.parse::<Tz>().ok())
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

        use crate::{DefaultScalarValue, FromInputValue, InputValue};

        fn tz_input_test(raw: &'static str, expected: Option<Tz>) {
            let input = <InputValue<DefaultScalarValue>>::scalar(raw.to_string());
            let parsed: Option<Tz> = FromInputValue::from_input_value(&input);

            assert_eq!(parsed, expected);
        }

        #[test]
        fn europe_zone() {
            tz_input_test("Europe/London", Some(chrono_tz::Europe::London));
        }

        #[test]
        fn etc_minus() {
            tz_input_test("Etc/GMT-3", Some(chrono_tz::Etc::GMTMinus3));
        }

        mod invalid {
            use super::tz_input_test;

            #[test]
            fn forward_slash() {
                tz_input_test("Abc/Xyz", None);
            }

            #[test]
            fn number() {
                tz_input_test("8086", None);
            }

            #[test]
            fn no_forward_slash() {
                tz_input_test("AbcXyz", None);
            }
        }
    }
}
