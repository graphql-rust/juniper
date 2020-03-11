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

#[cfg(test)]
mod test {
    use crate::{value::DefaultScalarValue, InputValue};
    use bson::oid::ObjectId;

    #[test]
    fn objectid_from_input_value() {
        let raw = "53e37d08776f724e42000000";
        let input: InputValue<DefaultScalarValue> = InputValue::scalar(raw.to_string());

        let parsed: ObjectId = crate::FromInputValue::from_input_value(&input).unwrap();
        let id = ObjectId::with_string(raw).unwrap();

        assert_eq!(parsed, id);
    }
}
