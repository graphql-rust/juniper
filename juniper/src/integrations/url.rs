use url::Url;

use parser::ParseError;
use value::ParseScalarValue;
use Value;

graphql_scalar!(Url where Scalar = <S>{
    description: "Url"

    resolve(&self) -> Value {
        Value::string(self.as_str())
    }

    from_input_value(v: &InputValue) -> Option<Url> {
        v.as_string_value()
         .and_then(|s| Url::parse(s).ok())
    }

    from_str<'a>(value: ScalarToken<'a>) -> Result<S, ParseError<'a>> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
});

#[cfg(test)]
mod test {
    use url::Url;
    use value::DefaultScalarValue;

    #[test]
    fn url_from_input_value() {
        let raw = "https://example.net/";
        let input: ::InputValue<DefaultScalarValue> = ::InputValue::string(raw.to_string());

        let parsed: Url = ::FromInputValue::from_input_value(&input).unwrap();
        let url = Url::parse(raw).unwrap();

        assert_eq!(parsed, url);
    }
}
