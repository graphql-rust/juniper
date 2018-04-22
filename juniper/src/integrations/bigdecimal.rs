use bigdecimal::BigDecimal;

use Value;

graphql_scalar!(BigDecimal {
    description: "BigDecimal"

    resolve(&self) -> Value {
        Value::string(self.to_string())
    }

    from_input_value(v: &InputValue) -> Option<BigDecimal> {
        v.as_string_value()
         .and_then(|s| s.parse::<BigDecimal>().ok())
    }
});

#[cfg(test)]
mod test {
    use bigdecimal::BigDecimal;

    #[test]
    fn bigdecimal_from_input_value() {
        let raw = "-10.1910";
        let input = ::InputValue::String(raw.to_string());

        let parsed: BigDecimal = ::FromInputValue::from_input_value(&input).unwrap();
        let bigdecimal = raw.parse::<BigDecimal>().unwrap();

        assert_eq!(parsed, bigdecimal);
    }
}
