use std::time::{SystemTime, Duration};

use humantime::{parse_rfc3339_weak, format_rfc3339};
use humantime::{parse_duration, format_duration};

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

graphql_scalar!(Duration as "Duration" {
    description: "Duration in human-readable form, like '15min 2ms'"

    resolve(&self) -> Value {
        Value::string(format_duration(*self).to_string())
    }

    from_input_value(v: &InputValue) -> Option<Duration> {
        v.as_string_value().and_then(|s| {
            parse_duration(s).ok()
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

    fn duration(raw: &'static str) -> Duration {
        let input = ::InputValue::String(raw.to_string());
        ::FromInputValue::from_input_value(&input).unwrap()
    }

    #[test]
    fn datetime_utc_from_input_value() {
        datetime_utc_test("2014-11-28T21:00:09Z", 1417208409)
    }

    #[test]
    fn duration_test() {
        assert_eq!(duration("2min"), Duration::new(120, 0));
        assert_eq!(duration("2m 15sec 12ms"), Duration::new(135, 12_000_000));
    }

}
