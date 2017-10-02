/*!

# Supported types

| Rust Type               | JSON Serialization     | Notes                                     |
|-------------------------|------------------------|-------------------------------------------|
| `DateTime<FixedOffset>` | RFC3339 string         |                                           |
| `DateTime<Utc>`         | RFC3339 string         |                                           |
| `NaiveDate`             | RFC3339 string         |                                           |
| `NaiveDateTime`         | float (unix timestamp) | JSON numbers (i.e. IEEE doubles) are not  |
|                         |                        | precise enough for nanoseconds.           |
|                         |                        | Values will be truncated to microsecond   |
|                         |                        | resolution.                               |

*/
use chrono::prelude::*;

use ::Value;

#[doc(hidden)]
pub static RFC3339_FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S%z";

graphql_scalar!(DateTime<FixedOffset> {
    description: "DateTime"

    resolve(&self) -> Value {
        Value::string(self.to_rfc3339())
    }

    from_input_value(v: &InputValue) -> Option<DateTime<FixedOffset>> {
        v.as_string_value()
         .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
    }
});

graphql_scalar!(DateTime<Utc> {
    description: "DateTime"

    resolve(&self) -> Value {
        Value::string(self.to_rfc3339())
    }

    from_input_value(v: &InputValue) -> Option<DateTime<Utc>> {
        v.as_string_value()
         .and_then(|s| (s.parse::<DateTime<Utc>>().ok()))
    }
});

// Don't use `Date` as the docs say:
// "[Date] should be considered ambiguous at best, due to the "
// inherent lack of precision required for the time zone resolution.
// For serialization and deserialization uses, it is best to use
// `NaiveDate` instead."
graphql_scalar!(NaiveDate {
    description: "NaiveDate"

    resolve(&self) -> Value {
        Value::string(self.format(RFC3339_FORMAT).to_string())
    }

    from_input_value(v: &InputValue) -> Option<NaiveDate> {
        v.as_string_value()
         .and_then(|s| NaiveDate::parse_from_str(s, RFC3339_FORMAT).ok())
    }
});

/// JSON numbers (i.e. IEEE doubles) are not precise enough for nanosecond
/// datetimes. Values will be truncated to microsecond resolution.
graphql_scalar!(NaiveDateTime {
    description: "NaiveDateTime"

    resolve(&self) -> Value {
        Value::float(self.timestamp() as f64)
    }

    from_input_value(v: &InputValue) -> Option<NaiveDateTime> {
        v.as_float_value()
         .and_then(|f| NaiveDateTime::from_timestamp_opt(f as i64, 0))
    }
});

#[cfg(test)]
mod test {
    use chrono::prelude::*;
    use super::RFC3339_FORMAT;

    #[test]
    fn datetime_fixedoffset_from_input_value() {
        let raw = "2014-11-28T21:00:09+09:00";
        let input = ::InputValue::String(raw.to_string());

        let parsed: DateTime<FixedOffset> = ::FromInputValue::from_input_value(&input).unwrap();
        let expected = DateTime::parse_from_rfc3339(raw).unwrap();

        assert_eq!(parsed, expected);
    }

    #[test]
    fn datetime_utc_from_input_value() {
        let raw = "2014-11-28T21:00:09+09:00";
        let input = ::InputValue::String(raw.to_string());

        let parsed: DateTime<Utc> = ::FromInputValue::from_input_value(&input).unwrap();
        let expected = DateTime::parse_from_rfc3339(raw).unwrap().with_timezone(&Utc);

        assert_eq!(parsed, expected);
    }

    #[test]
    fn naivedate_from_input_value() {
        let raw = "1996-12-19T16:39:57-08:00";
        let input = ::InputValue::String(raw.to_string());

        let parsed: NaiveDate = ::FromInputValue::from_input_value(&input).unwrap();
        let expected = NaiveDate::parse_from_str(raw, &RFC3339_FORMAT).unwrap();
        let expected_via_datetime = DateTime::parse_from_rfc3339(raw).unwrap().date().naive_utc();
        let expected_via_ymd = NaiveDate::from_ymd(1996, 12, 19);

        assert_eq!(parsed, expected);
        assert_eq!(parsed, expected_via_datetime);
        assert_eq!(parsed, expected_via_ymd);

        assert_eq!(parsed.year(), 1996);
        assert_eq!(parsed.month(), 12);
        assert_eq!(parsed.day(), 19);
    }

    #[test]
    fn naivedatetime_from_input_value() {
        let raw = 1_000_000_000_f64;
        let input = ::InputValue::Float(raw);

        let parsed: NaiveDateTime = ::FromInputValue::from_input_value(&input).unwrap();
        let expected = NaiveDateTime::from_timestamp_opt(raw as i64, 0).unwrap();

        assert_eq!(parsed, expected);
        assert_eq!(raw, expected.timestamp() as f64);

    }
}
