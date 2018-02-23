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

use Value;

#[doc(hidden)]
pub static RFC3339_FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S%.f%:z";
static RFC3339_PARSE_FORMAT: &'static str = "%+";

graphql_scalar!(DateTime<FixedOffset> as "DateTimeFixedOffset" {
    description: "DateTime"

    resolve(&self) -> Value {
        Value::string(self.to_rfc3339())
    }

    from_input_value(v: &InputValue) -> Option<DateTime<FixedOffset>> {
        v.as_string_value()
         .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
    }
});

graphql_scalar!(DateTime<Utc> as "DateTimeUtc" {
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
        Value::string(self.format("%Y-%m-%d").to_string())
    }

    from_input_value(v: &InputValue) -> Option<NaiveDate> {
        v.as_string_value()
         .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
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
    use super::RFC3339_PARSE_FORMAT;
    use chrono::prelude::*;
    use Value;

    fn datetime_fixedoffset_test(raw: &'static str) {
        let input = ::InputValue::String(raw.to_string());

        let parsed: DateTime<FixedOffset> = ::FromInputValue::from_input_value(&input).unwrap();
        let expected = DateTime::parse_from_rfc3339(raw).unwrap();

        assert_eq!(parsed, expected);
    }

    #[test]
    fn datetime_fixedoffset_from_input_value() {
        datetime_fixedoffset_test("2014-11-28T21:00:09+09:00");
    }

    #[test]
    fn datetime_fixedoffset_from_input_value_with_z_timezone() {
        datetime_fixedoffset_test("2014-11-28T21:00:09Z");
    }

    #[test]
    fn datetime_fixedoffset_from_input_value_with_fractional_seconds() {
        datetime_fixedoffset_test("2014-11-28T21:00:09.05+09:00");
    }

    fn datetime_utc_test(raw: &'static str) {
        let input = ::InputValue::String(raw.to_string());

        let parsed: DateTime<Utc> = ::FromInputValue::from_input_value(&input).unwrap();
        let expected = DateTime::parse_from_rfc3339(raw)
            .unwrap()
            .with_timezone(&Utc);

        assert_eq!(parsed, expected);
    }

    #[test]
    fn datetime_utc_from_input_value() {
        datetime_utc_test("2014-11-28T21:00:09+09:00")
    }

    #[test]
    fn datetime_utc_from_input_value_with_z_timezone() {
        datetime_utc_test("2014-11-28T21:00:09Z")
    }

    #[test]
    fn datetime_utc_from_input_value_with_fractional_seconds() {
        datetime_utc_test("2014-11-28T21:00:09.005+09:00");
    }

    fn naivedate_test(raw: &'static str, y: i32, m: u32, d: u32) {
        let input = ::InputValue::String(raw.to_string());

        let parsed: NaiveDate = ::FromInputValue::from_input_value(&input).unwrap();
        let expected = NaiveDate::from_ymd(y, m, d);

        assert_eq!(parsed, expected);

        assert_eq!(parsed.year(), y);
        assert_eq!(parsed.month(), m);
        assert_eq!(parsed.day(), d);
    }

    #[test]
    fn naivedate_from_input_value() {
        naivedate_test("1996-12-19", 1996, 12, 19);
    }

    #[test]
    fn naivedate_serialization() {
        use types::base::GraphQLType;

        let date = NaiveDate::from_ymd(2000, 1, 2);
        let value = date.resolve(&(), None, &executor::Executor);
        let expected = Value::string("2000-01-02".to_string());
        assert_eq!(value, expected);
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
