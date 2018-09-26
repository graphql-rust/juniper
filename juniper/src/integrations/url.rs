use url::Url;

use value::{ParseScalarResult, ParseScalarValue};
use Value;

graphql_scalar!(Url where Scalar = <S>{
    description: "Url"

    resolve(&self) -> Value {
        Value::scalar(self.as_str().to_owned())
    }

    from_input_value(v: &InputValue) -> Option<Url> {
        v.as_scalar_value::<String>()
         .and_then(|s| Url::parse(s).ok())
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
});

#[cfg(test)]
mod test {
    use url::Url;

    #[test]
    fn url_from_input_value() {
        let raw = "https://example.net/";
        let input: ::InputValue = ::InputValue::scalar(raw.to_string());

        let parsed: Url = ::FromInputValue::from_input_value(&input).unwrap();
        let url = Url::parse(raw).unwrap();

        assert_eq!(parsed, url);
    }
}
