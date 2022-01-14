//! GraphQL support for [bson](https://github.com/mongodb/bson-rust) types.

use bson::{oid::ObjectId, DateTime as UtcDateTime};
use chrono::prelude::*;

use crate::{
    graphql_scalar,
    parser::{ParseError, Token},
    value::ParseScalarResult,
    GraphQLScalar, InputValue, ScalarToken, ScalarValue, Value,
};

#[graphql_scalar(description = "ObjectId")]
impl<S: ScalarValue> GraphQLScalar<S> for ObjectId {
    type Error = String;

    fn to_output(&self) -> Value<S> {
        Value::scalar(self.to_hex())
    }

    fn from_input(v: &InputValue<S>) -> Result<Self, Self::Error> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                Self::parse_str(s).map_err(|e| format!("Failed to parse `ObjectId`: {}", e))
            })
    }

    fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
        if let ScalarToken::String(val) = value {
            Ok(S::from(val.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

#[graphql_scalar(description = "UtcDateTime")]
impl<S: ScalarValue> GraphQLScalar<S> for UtcDateTime {
    type Error = String;

    fn to_output(&self) -> Value<S> {
        Value::scalar((*self).to_chrono().to_rfc3339())
    }

    fn from_input(v: &InputValue<S>) -> Result<Self, Self::Error> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                s.parse::<DateTime<Utc>>()
                    .map_err(|e| format!("Failed to parse `UtcDateTime`: {}", e))
            })
            .map(Self::from_chrono)
    }

    fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
        if let ScalarToken::String(val) = value {
            Ok(S::from(val.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

#[cfg(test)]
mod test {
    use bson::{oid::ObjectId, DateTime as UtcDateTime};
    use chrono::{DateTime, Utc};

    use crate::{graphql_input_value, FromInputValue, InputValue};

    #[test]
    fn objectid_from_input_value() {
        let raw = "53e37d08776f724e42000000";
        let input: InputValue = graphql_input_value!((raw));

        let parsed: ObjectId = FromInputValue::from_input_value(&input).unwrap();
        let id = ObjectId::parse_str(raw).unwrap();

        assert_eq!(parsed, id);
    }

    #[test]
    fn utcdatetime_from_input_value() {
        let raw = "2020-03-23T17:38:32.446+00:00";
        let input: InputValue = graphql_input_value!((raw));

        let parsed: UtcDateTime = FromInputValue::from_input_value(&input).unwrap();
        let date_time = UtcDateTime::from_chrono(
            DateTime::parse_from_rfc3339(raw)
                .unwrap()
                .with_timezone(&Utc),
        );

        assert_eq!(parsed, date_time);
    }
}
