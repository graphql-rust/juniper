use uuid::Uuid;

use parser::{ParseError, ScalarToken, Token};
use value::ParseScalarResult;
use Value;

graphql_scalar!(Uuid where Scalar = <S> {
    description: "Uuid"

    resolve(&self) -> Value {
        Value::scalar(self.to_string())
    }

    from_input_value(v: &InputValue) -> Option<Uuid> {
        v.as_scalar_value::<String>()
         .and_then(|s| Uuid::parse_str(s).ok())
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(value) = value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
});

#[cfg(test)]
mod test {
    use uuid::Uuid;
    use value::DefaultScalarValue;

    #[test]
    fn uuid_from_input_value() {
        let raw = "123e4567-e89b-12d3-a456-426655440000";
        let input: ::InputValue<DefaultScalarValue> = ::InputValue::scalar(raw.to_string());

        let parsed: Uuid = ::FromInputValue::from_input_value(&input).unwrap();
        let id = Uuid::parse_str(raw).unwrap();

        assert_eq!(parsed, id);
    }
}
