//! GraphQL support for [uuid](https://doc.rust-lang.org/uuid/uuid/struct.Uuid.html) types.

#![allow(clippy::needless_lifetimes)]

use uuid::Uuid;

use crate::{
    parser::{ParseError, ScalarToken, Token},
    value::ParseScalarResult,
    GraphQLScalar, InputValue, ScalarValue, Value,
};

#[crate::graphql_scalar(description = "Uuid")]
impl<S: ScalarValue> GraphQLScalar<S> for Uuid {
    type Error = String;

    fn resolve(&self) -> Value<S> {
        Value::scalar(self.to_string())
    }

    fn from_input_value(v: &InputValue<S>) -> Result<Uuid, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| Uuid::parse_str(s).map_err(|e| format!("Failed to parse `Uuid`: {}", e)))
    }

    fn from_str(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
        if let ScalarToken::String(value) = value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

#[cfg(test)]
mod test {
    use uuid::Uuid;

    use crate::{graphql_input_value, FromInputValue, InputValue};

    #[test]
    fn uuid_from_input_value() {
        let raw = "123e4567-e89b-12d3-a456-426655440000";
        let input: InputValue = graphql_input_value!((raw));

        let parsed: Uuid = FromInputValue::from_input_value(&input).unwrap();
        let id = Uuid::parse_str(raw).unwrap();

        assert_eq!(parsed, id);
    }
}
