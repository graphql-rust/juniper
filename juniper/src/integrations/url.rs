//! GraphQL support for [url](https://github.com/servo/rust-url) types.

use crate::{graphql_scalar, InputValue, ScalarValue, Value};

#[graphql_scalar(with = url_scalar, parse_token(String))]
type Url = url::Url;

mod url_scalar {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &Url) -> Value<S> {
        Value::scalar(v.as_str().to_owned())
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Url, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| Url::parse(s).map_err(|e| format!("Failed to parse `Url`: {e}")))
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::{graphql_input_value, InputValue};

    #[test]
    fn url_from_input() {
        let raw = "https://example.net/";
        let input: InputValue = graphql_input_value!((raw));

        let parsed: Url = crate::FromInputValue::from_input_value(&input).unwrap();
        let url = Url::parse(raw).unwrap();

        assert_eq!(parsed, url);
    }
}
