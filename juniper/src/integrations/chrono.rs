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
#![allow(clippy::needless_lifetimes)]
use chrono::prelude::*;

use crate::{
    parser::{ParseError, ScalarToken, Token},
    value::{ParseScalarResult, ParseScalarValue},
    Value,
};

#[doc(hidden)]
pub static RFC3339_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.f%:z";

#[crate::graphql_scalar_internal(name = "DateTimeFixedOffset", description = "DateTime")]
impl<S> GraphQLScalar for DateTime<FixedOffset>
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.to_rfc3339())
    }

    fn from_input_value(v: &InputValue) -> Option<DateTime<FixedOffset>> {
        v.as_string_value()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(value) = value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

#[crate::graphql_scalar_internal(name = "DateTimeUtc", description = "DateTime")]
impl<S> GraphQLScalar for DateTime<Utc>
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.to_rfc3339())
    }

    fn from_input_value(v: &InputValue) -> Option<DateTime<Utc>> {
        v.as_string_value()
            .and_then(|s| (s.parse::<DateTime<Utc>>().ok()))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(value) = value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

// Don't use `Date` as the docs say:
// "[Date] should be considered ambiguous at best, due to the "
// inherent lack of precision required for the time zone resolution.
// For serialization and deserialization uses, it is best to use
// `NaiveDate` instead."
#[crate::graphql_scalar_internal(description = "NaiveDate")]
impl<S> GraphQLScalar for NaiveDate
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.format("%Y-%m-%d").to_string())
    }

    fn from_input_value(v: &InputValue) -> Option<NaiveDate> {
        v.as_string_value()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(value) = value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

// JSON numbers (i.e. IEEE doubles) are not precise enough for nanosecond
// datetimes. Values will be truncated to microsecond resolution.
#[crate::graphql_scalar_internal(description = "NaiveDateTime")]
impl<S> GraphQLScalar for NaiveDateTime
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.timestamp() as f64)
    }

    fn from_input_value(v: &InputValue) -> Option<NaiveDateTime> {
        v.as_float_value()
            .and_then(|f| NaiveDateTime::from_timestamp_opt(f as i64, 0))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <f64 as ParseScalarValue<S>>::from_str(value)
    }
}

#[cfg(test)]
mod test {
    use crate::{value::DefaultScalarValue, InputValue};
    use chrono::prelude::*;

    fn datetime_fixedoffset_test(raw: &'static str) {
        let input: crate::InputValue<DefaultScalarValue> = InputValue::scalar(raw.to_string());

        let parsed: DateTime<FixedOffset> =
            crate::FromInputValue::from_input_value(&input).unwrap();
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
        let input: crate::InputValue<DefaultScalarValue> = InputValue::scalar(raw.to_string());

        let parsed: DateTime<Utc> = crate::FromInputValue::from_input_value(&input).unwrap();
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
        let input: crate::InputValue<DefaultScalarValue> =
            InputValue::scalar("1996-12-19".to_string());
        let y = 1996;
        let m = 12;
        let d = 19;

        let parsed: NaiveDate = crate::FromInputValue::from_input_value(&input).unwrap();
        let expected = NaiveDate::from_ymd(y, m, d);

        assert_eq!(parsed, expected);

        assert_eq!(parsed.year(), y);
        assert_eq!(parsed.month(), m);
        assert_eq!(parsed.day(), d);
    }

    #[test]
    fn naivedatetime_from_input_value() {
        let raw = 1_000_000_000_f64;
        let input: InputValue<DefaultScalarValue> = InputValue::scalar(raw);

        let parsed: NaiveDateTime = crate::FromInputValue::from_input_value(&input).unwrap();
        let expected = NaiveDateTime::from_timestamp_opt(raw as i64, 0).unwrap();

        assert_eq!(parsed, expected);
        assert_eq!(raw, expected.timestamp() as f64);
    }
}

#[cfg(test)]
mod integration_test {
    use chrono::{prelude::*, Utc};

    use crate::{
        executor::Variables,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
        value::Value,
    };

    #[tokio::test]
    async fn test_serialization() {
        struct Root;

        #[crate::graphql_object_internal]
        impl Root {
            fn exampleNaiveDate() -> NaiveDate {
                NaiveDate::from_ymd(2015, 3, 14)
            }
            fn exampleNaiveDateTime() -> NaiveDateTime {
                NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11)
            }
            fn exampleDateTimeFixedOffset() -> DateTime<FixedOffset> {
                DateTime::parse_from_rfc3339("1996-12-19T16:39:57-08:00").unwrap()
            }
            fn exampleDateTimeUtc() -> DateTime<Utc> {
                Utc.timestamp(61, 0)
            }
        }

        let doc = r#"
        {
            exampleNaiveDate,
            exampleNaiveDateTime,
            exampleDateTimeFixedOffset,
            exampleDateTimeUtc,
        }
        "#;

        let schema = RootNode::new(
            Root,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );

        let (result, errs) = crate::execute(doc, None, &schema, &Variables::new(), &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        assert_eq!(
            result,
            Value::object(
                vec![
                    ("exampleNaiveDate", Value::scalar("2015-03-14")),
                    ("exampleNaiveDateTime", Value::scalar(1_467_969_011.0)),
                    (
                        "exampleDateTimeFixedOffset",
                        Value::scalar("1996-12-19T16:39:57-08:00"),
                    ),
                    (
                        "exampleDateTimeUtc",
                        Value::scalar("1970-01-01T00:01:01+00:00"),
                    ),
                ]
                .into_iter()
                .collect()
            )
        );
    }
}
