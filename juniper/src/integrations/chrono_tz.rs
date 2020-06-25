/*!

Adds Tz as a scalar represented by its database name

*/
use chrono_tz::Tz;

use crate::{
    parser::{ParseError, ScalarToken, Token},
    value::ParseScalarResult,
    Value,
};

graphql_scalar!(Tz as "Tz" where Scalar = <S> {
    description: "Tz"

    resolve(&self) -> Value {
        Value::scalar(self.name().to_owned())
    }

    from_input_value(v: &InputValue) -> Option<Tz> {
        v.as_string_value()
         .and_then(|s| s.parse::<Tz>().ok())
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(value) =  value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
});

#[cfg(test)]
mod test {
    use crate::{value::DefaultScalarValue, InputValue};
    use chrono_tz::Tz;

    fn tz_input_test(raw: &'static str, expected: Option<Tz>) {
        let input: crate::InputValue<DefaultScalarValue> = InputValue::scalar(raw.to_string());

        let parsed: Option<Tz> = crate::FromInputValue::from_input_value(&input);

        assert_eq!(parsed, expected);
    }

    #[test]
    fn tz_from_input_value_europe_zone() {
        tz_input_test("Europe/London", Some(chrono_tz::Europe::London));
    }

    #[test]
    fn tz_from_input_value_etc_minus() {
        tz_input_test("Etc/GMT-3", Some(chrono_tz::Etc::GMTMinus3));
    }

    #[test]
    fn tz_from_input_value_invalid_with_forward_slash() {
        tz_input_test("Abc/Xyz", None);
    }

    #[test]
    fn tz_from_input_value_invalid_with_number() {
        tz_input_test("8086", None);
    }

    #[test]
    fn tz_from_input_value_invalid_with_no_forward_slash() {
        tz_input_test("AbcXyz", None);
    }
}
