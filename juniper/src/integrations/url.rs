use url::Url;

use Value;

graphql_scalar!(Url {
    description: "Url"

    resolve(&self) -> Value {
        Value::string(self.as_str())
    }

    from_input_value(v: &InputValue) -> Option<Url> {
        v.as_string_value()
         .and_then(|s| Url::parse(s).ok())
    }
});

#[cfg(test)]
mod test {
    use url::Url;

    #[test]
    fn url_from_input_value() {
        let raw = "https://example.net/";
        let input = ::InputValue::String(raw.to_string());

        let parsed: Url = ::FromInputValue::from_input_value(&input).unwrap();
        let url = Url::parse(raw).unwrap();

        assert_eq!(parsed, url);
    }
}
