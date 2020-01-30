use bson::oid::ObjectId;

use crate::{
    parser::{ParseError, ScalarToken, Token},
    value::ParseScalarResult,
    Value,
};

graphql_scalar!(ObjectId where Scalar = <S> {
    description: "ObjectId"

    resolve(&self) -> Value {
        Value::scalar(self.to_hex())
    }

    from_input_value(v: &InputValue) -> Option<ObjectId> {
        v.as_string_value()
         .and_then(|s| ObjectId::with_string(s).ok())
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(value) = value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
});
