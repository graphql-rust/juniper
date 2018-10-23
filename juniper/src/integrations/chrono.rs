/*!

# Supported types

| Rust Type               | JSON Serialization     | Notes                                     |
|-------------------------|------------------------|-------------------------------------------|
| `DateTime<FixedOffset>` | RFC3339 string         |                                           |
| `DateTime<Utc>`         | RFC3339 string         |                                           |
| `NaiveDate`             | YYYY-MM-DD             |                                           |
| `NaiveDateTime`         | float (unix timestamp) | JSON numbers (i.e. IEEE doubles) are not  |
|                         |                        | precise enough for nanoseconds.           |
|                         |                        | Values will be truncated to microsecond   |
|                         |                        | resolution.                               |

*/
use chrono::prelude::*;

use parser::{ParseError, ScalarToken, Token};
use value::{ParseScalarResult, ParseScalarValue};
use Value;

#[doc(hidden)]
pub static RFC3339_FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S%.f%:z";

graphql_scalar!(DateTime<FixedOffset> as "DateTimeFixedOffset" where Scalar = <S>{
    description: "DateTime"

    resolve(&self) -> Value {
        Value::scalar(self.to_rfc3339())
    }

    from_input_value(v: &InputValue) -> Option<DateTime<FixedOffset>> {
        v.as_scalar_value::<String>()
         .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(value) =  value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
});

graphql_scalar!(DateTime<Utc> as "DateTimeUtc" where Scalar = <S>{
    description: "DateTime"

    resolve(&self) -> Value {
        Value::scalar(self.to_rfc3339())
    }

    from_input_value(v: &InputValue) -> Option<DateTime<Utc>> {
        v.as_scalar_value::<String>()
         .and_then(|s| (s.parse::<DateTime<Utc>>().ok()))
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(value) = value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
});

// Don't use `Date` as the docs say:
// "[Date] should be considered ambiguous at best, due to the "
// inherent lack of precision required for the time zone resolution.
// For serialization and deserialization uses, it is best to use
// `NaiveDate` instead."
graphql_scalar!(NaiveDate where Scalar = <S>{
    description: "NaiveDate"

    resolve(&self) -> Value {
        Value::scalar(self.format("%Y-%m-%d").to_string())
    }

    from_input_value(v: &InputValue) -> Option<NaiveDate> {
        v.as_scalar_value::<String>()
         .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(value) = value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
});

/// JSON numbers (i.e. IEEE doubles) are not precise enough for nanosecond
/// datetimes. Values will be truncated to microsecond resolution.
graphql_scalar!(NaiveDateTime where Scalar = <S> {
    description: "NaiveDateTime"

    resolve(&self) -> Value {
        Value::scalar(self.timestamp() as f64)
    }

    from_input_value(v: &InputValue) -> Option<NaiveDateTime> {
        v.as_scalar_value::<f64>()
         .and_then(|f| NaiveDateTime::from_timestamp_opt(*f as i64, 0))
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <f64 as ParseScalarValue<S>>::from_str(value)
    }
});

#[cfg(test)]
mod test {
    use chrono::prelude::*;
    use value::DefaultScalarValue;

    fn datetime_fixedoffset_test(raw: &'static str) {
        let input: ::InputValue<DefaultScalarValue> = ::InputValue::scalar(raw.to_string());

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
        let input: ::InputValue<DefaultScalarValue> = ::InputValue::scalar(raw.to_string());

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

    #[test]
    fn naivedate_from_input_value() {
        let input: ::InputValue<DefaultScalarValue> =
            ::InputValue::scalar("1996-12-19".to_string());
        let y = 1996;
        let m = 12;
        let d = 19;

        let parsed: NaiveDate = ::FromInputValue::from_input_value(&input).unwrap();
        let expected = NaiveDate::from_ymd(y, m, d);

        assert_eq!(parsed, expected);

        assert_eq!(parsed.year(), y);
        assert_eq!(parsed.month(), m);
        assert_eq!(parsed.day(), d);
    }

    #[test]
    fn naivedatetime_from_input_value() {
        let raw = 1_000_000_000_f64;
        let input: ::InputValue<DefaultScalarValue> = ::InputValue::scalar(raw);

        let parsed: NaiveDateTime = ::FromInputValue::from_input_value(&input).unwrap();
        let expected = NaiveDateTime::from_timestamp_opt(raw as i64, 0).unwrap();

        assert_eq!(parsed, expected);
        assert_eq!(raw, expected.timestamp() as f64);
    }
}

#[cfg(test)]
mod integration_test {
    use chrono::prelude::*;
    use chrono::Utc;

    use executor::Variables;
    use schema::model::RootNode;
    use types::scalars::EmptyMutation;
    use value::{Value};

    #[test]
    fn test_serialization() {
        struct Root;
        graphql_object!(Root: () |&self| {
            field exampleNaiveDate() -> NaiveDate {
                NaiveDate::from_ymd(2015, 3, 14)
            }
            field exampleNaiveDateTime() -> NaiveDateTime {
                NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11)
            }
            field exampleDateTimeFixedOffset() -> DateTime<FixedOffset> {
              DateTime::parse_from_rfc3339("1996-12-19T16:39:57-08:00").unwrap()
            }
            field exampleDateTimeUtc() -> DateTime<Utc> {
              Utc.timestamp(61, 0)
            }
        });

        let doc = r#"
        {
            exampleNaiveDate,
            exampleNaiveDateTime,
            exampleDateTimeFixedOffset,
            exampleDateTimeUtc,
        }
        "#;

        let schema= RootNode::new(Root, EmptyMutation::<()>::new());

        let (result, errs) =
            ::execute(doc, None, &schema, &Variables::new(), &()).expect("Execution failed");

        assert_eq!(errs, []);

        assert_eq!(
            result,
            Value::object(
                vec![
                    ("exampleNaiveDate", Value::scalar("2015-03-14")),
                    ("exampleNaiveDateTime", Value::scalar(1467969011.0)),
                    (
                        "exampleDateTimeFixedOffset",
                        Value::scalar("1996-12-19T16:39:57-08:00"),
                    ),
                    (
                        "exampleDateTimeUtc",
                        Value::scalar("1970-01-01T00:01:01+00:00"),
                    ),
                ].into_iter()
                .collect()
            )
        );
    }
}
