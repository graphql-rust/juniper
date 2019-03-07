use serde_json::Value as JsonValue;

use parser::{ParseError, ScalarToken, Token};
use value::ParseScalarResult;
use Value;

graphql_scalar!(JsonValue as "JsonString" where Scalar = <S> {
    description: "JSON serialized as a string"

    resolve(&self) -> Value {
        Value::scalar(self.to_string())
    }

    from_input_value(v: &InputValue) -> Option<JsonValue> {
        v.as_scalar_value::<String>()
         .and_then(|s| serde_json::from_str(s).ok())
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
    use super::*;

    #[test]
    fn json_from_input_value() {
        let raw = r#"{ "foo": "bar"}"#;
        let input: ::InputValue = ::InputValue::scalar(raw.to_string());

        let parsed: JsonValue = ::FromInputValue::from_input_value(&input).unwrap();
        let expected: JsonValue = serde_json::from_str(raw).unwrap();

        assert_eq!(parsed, expected);
    }

}

#[cfg(test)]
mod integration_test {
    use super::*;

    use executor::Variables;
    use schema::model::RootNode;
    use types::scalars::EmptyMutation;
    use value::Value;

    #[test]
    fn test_json_serialization() {
        let example_raw: JsonValue = serde_json::from_str(
            r#"{
            "x": 2,
            "y": 42
            }
        "#,
        )
        .unwrap();
        let example_raw = example_raw.to_string();

        struct Root;
        graphql_object!(Root: () |&self| {
            field example_json() -> JsonValue {
                serde_json::from_str(r#"{
                    "x": 2,
                    "y": 42
                    }
                "#).unwrap()
            }
            field input_json(input: JsonValue) -> bool {
                input.is_array()
            }
        });

        let doc = r#"
        {
            exampleJson,
            inputJson(input: "[]"),
        }
        "#;

        let schema = RootNode::new(Root, EmptyMutation::<()>::new());

        let (result, errs) =
            ::execute(doc, None, &schema, &Variables::new(), &()).expect("Execution failed");

        assert_eq!(errs, []);

        assert_eq!(
            result,
            Value::object(
                vec![
                    ("exampleJson", Value::scalar(example_raw)),
                    ("inputJson", Value::scalar(true)),
                ]
                .into_iter()
                .collect()
            )
        );
    }
}
