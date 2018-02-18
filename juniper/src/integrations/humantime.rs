use std::time::{SystemTime};

use humantime::{parse_rfc3339_weak, format_rfc3339};

use Value;

graphql_scalar!(SystemTime as "DateTime" {
    description: "DateTime represented as a RFC3339 string (in UTC)"

    resolve(&self) -> Value {
        Value::string(format_rfc3339(*self).to_string())
    }

    from_input_value(v: &InputValue) -> Option<SystemTime> {
        v.as_string_value().and_then(|s| {
            // Uses permissive parser for input
            parse_rfc3339_weak(s).ok()
        })
    }
});


#[cfg(test)]
mod test {
    use std::time::{SystemTime, Duration, UNIX_EPOCH};

    fn datetime_utc_test(raw: &'static str, sec: u64) {
        let input = ::InputValue::String(raw.to_string());

        let parsed: SystemTime = ::FromInputValue::from_input_value(&input).unwrap();
        let expected: SystemTime = UNIX_EPOCH + Duration::new(sec, 0);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn datetime_utc_from_input_value() {
        datetime_utc_test("2014-11-28T21:00:09Z", 1417208409)
    }

}
