#![allow(clippy::needless_lifetimes)]

use uuid::Uuid;

use crate::{
    parser::{ParseError, ScalarToken, Token},
    value::ParseScalarResult,
    Value,
};

#[crate::graphql_scalar_internal(description = "Uuid")]
impl<S> GraphQLScalar for Uuid
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.to_string())
    }

    fn from_input_value(v: &InputValue) -> Option<Uuid> {
        v.as_string_value().and_then(|s| Uuid::parse_str(s).ok())
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(value) = value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{value::DefaultScalarValue, InputValue};
    use uuid::Uuid;

    #[test]
    fn uuid_from_input_value() {
        let raw = "123e4567-e89b-12d3-a456-426655440000";
        let input: InputValue<DefaultScalarValue> = InputValue::scalar(raw.to_string());

        let parsed: Uuid = crate::FromInputValue::from_input_value(&input).unwrap();
        let id = Uuid::parse_str(raw).unwrap();

        assert_eq!(parsed, id);
    }
}
