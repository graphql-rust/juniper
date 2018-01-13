use uuid::Uuid;

use Value;

graphql_scalar!(Uuid {
    description: "Uuid"

    resolve(&self) -> Value {
        Value::string(self.to_string())
    }

    from_input_value(v: &InputValue) -> Option<Uuid> {
        v.as_string_value()
         .and_then(|s| Uuid::parse_str(s).ok())
    }
});

#[cfg(test)]
mod test {
    use uuid::Uuid;

    #[test]
    fn uuid_from_input_value() {
        let raw = "123e4567-e89b-12d3-a456-426655440000";
        let input = ::InputValue::String(raw.to_string());

        let parsed: Uuid = ::FromInputValue::from_input_value(&input).unwrap();
        let id = Uuid::parse_str(raw).unwrap();

        assert_eq!(parsed, id);
    }
}
