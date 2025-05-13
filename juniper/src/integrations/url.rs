//! GraphQL support for [`url`] crate types.
//!
//! # Supported types
//!
//! | Rust type | GraphQL scalar |
//! |-----------|----------------|
//! | [`Url`]   | [`URL`][s1]    |
//!
//! [`Url`]: url::Url
//! [s1]: https://graphql-scalars.dev/docs/scalars/url

use crate::{InputValue, ScalarValue, Value, graphql_scalar};

/// [Standard URL][0] format as specified in [RFC 3986].
///
/// [`URL` scalar][1] compliant.
///
/// See also [`url::Url`][2] for details.
///
/// [0]: http://url.spec.whatwg.org
/// [1]: https://graphql-scalars.dev/docs/scalars/url
/// [2]: https://docs.rs/url/*/url/struct.Url.html
/// [RFC 3986]: https://datatracker.ietf.org/doc/html/rfc3986
#[graphql_scalar(
    name = "URL",
    with = url_scalar,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/url",
)]
type Url = url::Url;

mod url_scalar {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &Url) -> Value<S> {
        Value::scalar(v.as_str().to_owned())
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Url, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| Url::parse(s).map_err(|e| format!("Failed to parse `URL`: {e}")))
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use crate::{InputValue, graphql_input_value};

    #[test]
    fn url_from_input() {
        let raw = "https://example.net/";
        let input: InputValue = graphql_input_value!((raw));

        let parsed: Url = crate::FromInputValue::from_input_value(&input).unwrap();
        let url = Url::parse(raw).unwrap();

        assert_eq!(parsed, url);
    }
}
