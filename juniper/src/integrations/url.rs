//! GraphQL support for [url](https://github.com/servo/rust-url) types.

use url::Url;

use crate::{
    value::{ParseScalarResult, ParseScalarValue},
    Value,
};

#[crate::graphql_scalar(description = "Url", scalar = S)]
impl<S> GraphQLScalar for Url {
    fn resolve(&self) -> Value {
        Value::scalar(self.as_str().to_owned())
    }

    fn from_input_value(v: &InputValue) -> Result<Url, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| Url::parse(s).map_err(|e| format!("Failed to parse `Url`: {}", e)))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
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
