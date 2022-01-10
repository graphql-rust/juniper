//! GraphQL support for [url](https://github.com/servo/rust-url) types.

use url::Url;

use crate::{
    value::{ParseScalarResult, ParseScalarValue},
    GraphQLScalar, InputValue, ScalarToken, ScalarValue, Value,
};

#[crate::graphql_scalar(description = "Url")]
impl<S: ScalarValue> GraphQLScalar<S> for Url {
    type Error = String;

    fn resolve(&self) -> Value<S> {
        Value::scalar(self.as_str().to_owned())
    }

    fn from_input_value(v: &InputValue<S>) -> Result<Url, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| Url::parse(s).map_err(|e| format!("Failed to parse `Url`: {}", e)))
    }

    fn from_str(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::{graphql_input_value, InputValue};

    #[test]
    fn url_from_input_value() {
        let raw = "https://example.net/";
        let input: InputValue = graphql_input_value!((raw));

        let parsed: Url = crate::FromInputValue::from_input_value(&input).unwrap();
        let url = Url::parse(raw).unwrap();

        assert_eq!(parsed, url);
    }
}
