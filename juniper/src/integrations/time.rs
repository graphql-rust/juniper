/*!

# Supported types

| Rust Type           | JSON Serialization  | Notes                                |
|---------------------|---------------------|--------------------------------------|
| `OffsetDateTime`    | RFC3339 string      |                                      |
| `Date`              | YYYY-MM-DD          |                                      |
| `PrimitiveDateTime` | YYYY-MM-DD HH-MM-SS |                                      |
| `Time`              | H:M:S               | Optional. Use the `scalar-naivetime` |
|                     |                     | feature.                             |

*/
use time::{
    format_description::parse, format_description::well_known::Rfc3339, Date, OffsetDateTime,
    PrimitiveDateTime,
};

#[cfg(feature = "scalar-naivetime")]
use time::Time;

use crate::{
    parser::{ParseError, ScalarToken, Token},
    value::{ParseScalarResult, ParseScalarValue},
    Value,
};

#[doc(hidden)]
pub static RFC3339_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.f%:z";

#[crate::graphql_scalar(name = "DateTimeFixedOffset", description = "OffsetDateTime")]
impl<S> GraphQLScalar for OffsetDateTime
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(
            self.format(&Rfc3339)
                .expect("Failed to format `DateTimeFixedOffset`"),
        )
    }

    fn from_input_value(v: &InputValue) -> Result<OffsetDateTime, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                time::OffsetDateTime::parse(s, &Rfc3339)
                    .map_err(|e| format!("Failed to parse `DateTimeFixedOffset`: {}", e))
            })
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(value) = value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

#[crate::graphql_scalar(description = "Date")]
impl<S> GraphQLScalar for Date
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        let description =
            parse("[year]-[month]-[day]").expect("Failed to parse format description");
        Value::scalar(
            self.format(&description)
                .expect("Failed to format `Date`")
                .to_string(),
        )
    }

    fn from_input_value(v: &InputValue) -> Result<Date, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                let description =
                    parse("[year]-[month]-[day]").expect("Failed to parse format description");
                Date::parse(s, &description).map_err(|e| format!("Failed to parse `Date`: {}", e))
            })
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(value) = value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

#[cfg(feature = "scalar-naivetime")]
#[crate::graphql_scalar(description = "Time")]
impl<S> GraphQLScalar for Time
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        let description =
            parse("[hour]:[minute]:[second]").expect("Failed to parse format description");
        Value::scalar(
            self.format(&description)
                .expect("Failed to format `Time`")
                .to_string(),
        )
    }

    fn from_input_value(v: &InputValue) -> Result<Time, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                let description =
                    parse("[hour]:[minute]:[second]").expect("Failed to parse format description");
                Time::parse(s, &description).map_err(|e| format!("Failed to parse `Time`: {}", e))
            })
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(value) = value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

#[crate::graphql_scalar(description = "PrimitiveDateTime")]
impl<S> GraphQLScalar for PrimitiveDateTime
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        let description = parse("[year]-[month]-[day] [hour]:[minute]:[second]")
            .expect("Failed to parse format description");
        Value::scalar(
            self.format(&description)
                .expect("Failed to format `PrimitiveDateTime`"),
        )
    }

    fn from_input_value(v: &InputValue) -> Result<PrimitiveDateTime, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                let description = parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                    .expect("Failed to parse format description");
                PrimitiveDateTime::parse(s, &description)
                    .map_err(|e| format!("Failed to parse `PrimitiveDateTime`: {}", e))
            })
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <f64 as ParseScalarValue<S>>::from_str(value)
    }
}

#[cfg(test)]
mod test {
    use std::convert::TryFrom;

    use time::{
        format_description::parse, format_description::well_known::Rfc3339, Date, Month,
        OffsetDateTime, PrimitiveDateTime, Time,
    };

    use crate::{graphql_input_value, FromInputValue, InputValue};

    fn offsetdatetime_test(raw: &'static str) {
        let input: InputValue = graphql_input_value!((raw));

        let parsed: OffsetDateTime = FromInputValue::from_input_value(&input).unwrap();
        let expected = OffsetDateTime::parse(raw, &Rfc3339).unwrap();

        assert_eq!(parsed, expected);
    }

    #[test]
    fn offsetdatetime_from_input_value() {
        offsetdatetime_test("2014-11-28T21:00:09+09:00");
    }

    #[test]
    fn offsetdatetime_from_input_value_with_z_timezone() {
        offsetdatetime_test("2014-11-28T21:00:09Z");
    }

    #[test]
    fn offsetdatetime_from_input_value_with_fractional_seconds() {
        offsetdatetime_test("2014-11-28T21:00:09.05+09:00");
    }

    #[test]
    fn date_from_input_value() {
        let y = 1996;
        let m = 12;
        let d = 19;
        let input: InputValue = graphql_input_value!("1996-12-19");

        let parsed: Date = FromInputValue::from_input_value(&input).unwrap();
        let expected = Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap();

        assert_eq!(parsed, expected);

        assert_eq!(parsed.year(), y);
        assert_eq!(u8::from(parsed.month()), m);
        assert_eq!(parsed.day(), d);
    }

    #[test]
    #[cfg(feature = "scalar-naivetime")]
    fn time_from_input_value() {
        let input: InputValue = graphql_input_value!("21:12:19");
        let [h, m, s] = [21, 12, 19];
        let parsed: Time = FromInputValue::from_input_value(&input).unwrap();
        let expected = Time::from_hms(h, m, s).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.hour(), h);
        assert_eq!(parsed.minute(), m);
        assert_eq!(parsed.second(), s);
    }

    #[test]
    fn primitivedatetime_from_input_value() {
        let raw = "2021-12-15 14:12:00";
        let input: InputValue = graphql_input_value!((raw));

        let parsed: PrimitiveDateTime = FromInputValue::from_input_value(&input).unwrap();
        let description = parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();
        let expected = PrimitiveDateTime::parse(&raw, &description).unwrap();

        assert_eq!(parsed, expected);
    }
}

#[cfg(test)]
mod integration_test {
    use time::{
        format_description::parse, format_description::well_known::Rfc3339, Date, Month,
        OffsetDateTime, PrimitiveDateTime,
    };

    #[cfg(feature = "scalar-naivetime")]
    use time::Time;

    use crate::{
        graphql_object, graphql_value, graphql_vars,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
    };

    #[tokio::test]
    async fn test_serialization() {
        struct Root;

        #[graphql_object]
        #[cfg(feature = "scalar-naivetime")]
        impl Root {
            fn example_date() -> Date {
                Date::from_calendar_date(2015, Month::March, 14).unwrap()
            }
            fn example_primitive_date_time() -> PrimitiveDateTime {
                let description = parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();
                PrimitiveDateTime::parse("2016-07-08 09:10:11", &description).unwrap()
            }
            fn example_time() -> Time {
                Time::from_hms(16, 7, 8).unwrap()
            }
            fn example_offset_date_time() -> OffsetDateTime {
                OffsetDateTime::parse("1996-12-19T16:39:57-08:00", &Rfc3339).unwrap()
            }
        }

        #[graphql_object]
        #[cfg(not(feature = "scalar-naivetime"))]
        impl Root {
            fn example_date() -> Date {
                Date::from_calendar_date(2015, Month::March, 14).unwrap()
            }
            fn example_primitive_date_time() -> PrimitiveDateTime {
                let description = parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();
                PrimitiveDateTime::parse("2016-07-08 09:10:11", &description).unwrap()
            }
            fn example_offset_date_time() -> OffsetDateTime {
                OffsetDateTime::parse("1996-12-19T16:39:57-08:00", &Rfc3339).unwrap()
            }
        }

        #[cfg(feature = "scalar-naivetime")]
        let doc = r#"{
            exampleDate,
            examplePrimitiveDateTime,
            exampleTime,
            exampleOffsetDateTime,
        }"#;

        #[cfg(not(feature = "scalar-naivetime"))]
        let doc = r#"{
            exampleDate,
            examplePrimitiveDateTime,
            exampleOffsetDateTime,
        }"#;

        let schema = RootNode::new(
            Root,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );

        let (result, errs) = crate::execute(doc, None, &schema, &graphql_vars! {}, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        #[cfg(feature = "scalar-naivetime")]
        assert_eq!(
            result,
            graphql_value!({
                "exampleDate": "2015-03-14",
                "examplePrimitiveDateTime": "2016-07-08 09:10:11",
                "exampleTime": "16:07:08",
                "exampleOffsetDateTime": "1996-12-19T16:39:57-08:00",
            }),
        );
        #[cfg(not(feature = "scalar-naivetime"))]
        assert_eq!(
            result,
            graphql_value!({
                "exampleDate": "2015-03-14",
                "examplePrimitiveDateTime": "2016-07-08 09:10:11",
                "exampleOffsetDateTime": "1996-12-19T16:39:57-08:00",
            }),
        );
    }
}
