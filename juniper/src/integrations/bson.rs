//! GraphQL support for [bson](https://github.com/mongodb/bson-rust) types.

use bson::{oid::ObjectId, DateTime as UtcDateTime};
use chrono::prelude::*;

use crate::{
    graphql_scalar,
    parser::{ParseError, ScalarToken, Token},
    value::ParseScalarResult,
    Value,
};

#[graphql_scalar(description = "ObjectId")]
impl<S> GraphQLScalar for ObjectId
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.to_hex())
    }

    fn from_input_value(v: &InputValue) -> Option<ObjectId> {
        v.as_string_value().and_then(|s| Self::parse_str(s).ok())
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(val) = value {
            Ok(S::from(val.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

#[graphql_scalar(description = "UtcDateTime")]
impl<S> GraphQLScalar for UtcDateTime
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar((*self).to_chrono().to_rfc3339())
    }

    fn from_input_value(v: &InputValue) -> Option<UtcDateTime> {
        v.as_string_value()
            .and_then(|s| (s.parse::<DateTime<Utc>>().ok()))
            .map(Self::from_chrono)
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
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

    use crate::{value::DefaultScalarValue, FromInputValue, InputValue};

    #[test]
    fn objectid_from_input_value() {
        let raw = "53e37d08776f724e42000000";
        let input = InputValue::<DefaultScalarValue>::scalar(raw.to_string());

        let parsed: ObjectId = FromInputValue::from_input_value(&input).unwrap();
        let id = ObjectId::parse_str(raw).unwrap();

        assert_eq!(parsed, id);
    }

    #[test]
    fn utcdatetime_from_input_value() {
        let raw = "2020-03-23T17:38:32.446+00:00";
        let input = InputValue::<DefaultScalarValue>::scalar(raw.to_string());

        let parsed: UtcDateTime = FromInputValue::from_input_value(&input).unwrap();
        let date_time = UtcDateTime::from_chrono(
            DateTime::parse_from_rfc3339(raw)
                .unwrap()
                .with_timezone(&Utc),
        );

        assert_eq!(parsed, date_time);
    }
}
