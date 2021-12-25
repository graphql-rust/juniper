//! GraphQL support for [`chrono`] crate types.
//!
//! # Supported types
//!
//! | Rust type         | Format                | GraphQL scalar    |
//! |-------------------|-----------------------|-------------------|
//! | [`NaiveDate`]     | `yyyy-MM-dd`          | [`Date`][s1]      |
//! | [`NaiveTime`]     | `HH:mm[:ss[.SSS]]`    | [`LocalTime`][s2] |
//! | [`NaiveDateTime`] | `yyyy-MM-dd HH:mm:ss` | `LocalDateTime`   |
//! | [`DateTime`]      | [RFC 3339] string     | [`DateTime`][s4]  |
//!
//! [`DateTime`]: chrono::DateTime
//! [`NaiveDate`]: chrono::naive::NaiveDate
//! [`NaiveDateTime`]: chrono::naive::NaiveDateTime
//! [`NaiveTime`]: chrono::naive::NaiveTime
//! [RFC 3339]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6
//! [s1]: https://graphql-scalars.dev/docs/scalars/date
//! [s2]: https://graphql-scalars.dev/docs/scalars/local-time
//! [s4]: https://graphql-scalars.dev/docs/scalars/date-time

use chrono::{SecondsFormat, Timelike as _};

use crate::{
    graphql_scalar,
    parser::{ParseError, ScalarToken, Token},
    value::ParseScalarResult,
    Value,
};

pub use chrono::{
    DateTime, FixedOffset as UtcOffset, NaiveDate as Date, NaiveDateTime as LocalDateTime,
    NaiveTime as LocalTime, Utc,
};

/// Format of a [`Date` scalar][1].
///
/// [1]: https://graphql-scalars.dev/docs/scalars/date
const DATE_FORMAT: &str = "%Y-%m-%d";

#[graphql_scalar(
    description = "Date in the proleptic Gregorian calendar (without time \
                   zone).\
                   \n\n\
                   Represents a description of the date (as used for birthdays,
                   for example). It cannot represent an instant on the \
                   time-line.\
                   \n\n\
                   [`Date` scalar][1] compliant.\
                   \n\n\
                   See also [`chrono::NaiveDate`][2] for details.\
                   \n\n\
                   [1]: https://graphql-scalars.dev/docs/scalars/date\n\
                   [2]: https://docs.rs/chrono/*/chrono/naive/struct.NaiveDate.html",
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/date"
)]
impl<S: ScalarValue> GraphQLScalar for Date {
    fn resolve(&self) -> Value {
        Value::scalar(self.format(DATE_FORMAT).to_string())
    }

    fn from_input_value(v: &InputValue) -> Result<Self, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                Self::parse_from_str(s, DATE_FORMAT).map_err(|e| format!("Invalid `Date`: {}", e))
            })
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(s) = value {
            Ok(S::from(s.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

/// Full format of a [`LocalTime` scalar][1].
///
/// [1]: https://graphql-scalars.dev/docs/scalars/local-time
const LOCAL_TIME_FORMAT: &str = "%H:%M:%S%.3f";

/// Format of a [`LocalTime` scalar][1] without milliseconds.
///
/// [1]: https://graphql-scalars.dev/docs/scalars/local-time
const LOCAL_TIME_FORMAT_NO_MILLIS: &str = "%H:%M:%S";

/// Format of a [`LocalTime` scalar][1] without seconds.
///
/// [1]: https://graphql-scalars.dev/docs/scalars/local-time
const LOCAL_TIME_FORMAT_NO_SECS: &str = "%H:%M";

#[graphql_scalar(
    description = "Clock time within a given date (without time zone) in \
                   `HH:mm[:ss[.SSS]]` format.\
                   \n\n\
                   All minutes are assumed to have exactly 60 seconds; no \
                   attempt is made to handle leap seconds (either positive or \
                   negative).\
                   \n\n\
                   [`LocalTime` scalar][1] compliant.\
                   \n\n\
                   See also [`chrono::NaiveTime`][2] for details.\
                   \n\n\
                   [1]: https://graphql-scalars.dev/docs/scalars/local-time\n\
                   [2]: https://docs.rs/chrono/*/chrono/naive/struct.NaiveTime.html",
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/local-time"
)]
impl<S: ScalarValue> GraphQLScalar for LocalTime {
    fn resolve(&self) -> Value {
        Value::scalar(
            if self.nanosecond() == 0 {
                self.format(LOCAL_TIME_FORMAT_NO_MILLIS)
            } else {
                self.format(LOCAL_TIME_FORMAT)
            }
            .to_string(),
        )
    }

    fn from_input_value(v: &InputValue) -> Result<Self, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                // First, try to parse the most used format.
                // At the end, try to parse the full format for the parsing
                // error to be most informative.
                Self::parse_from_str(s, LOCAL_TIME_FORMAT_NO_MILLIS)
                    .or_else(|_| Self::parse_from_str(s, LOCAL_TIME_FORMAT_NO_SECS))
                    .or_else(|_| Self::parse_from_str(s, LOCAL_TIME_FORMAT))
                    .map_err(|e| format!("Invalid `LocalTime`: {}", e))
            })
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(s) = value {
            Ok(S::from(s.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

/// Format of a `LocalDateTime` scalar.
const LOCAL_DATE_TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[graphql_scalar(
    description = "Combined date and time (without time zone) in `yyyy-MM-dd \
                   HH:mm:ss` format.\
                   \n\n\
                   See also [`chrono::NaiveDateTime`][1] for details.\
                   \n\n\
                   [1]: https://docs.rs/chrono/*/chrono/naive/struct.NaiveDateTime.html"
)]
impl<S: ScalarValue> GraphQLScalar for LocalDateTime {
    fn resolve(&self) -> Value {
        Value::scalar(self.format(LOCAL_DATE_TIME_FORMAT).to_string())
    }

    fn from_input_value(v: &InputValue) -> Result<Self, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                Self::parse_from_str(s, LOCAL_DATE_TIME_FORMAT)
                    .map_err(|e| format!("Invalid `LocalDateTime`: {}", e))
            })
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(s) = value {
            Ok(S::from(s.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

// TODO: Make generic over `chrono::TimeZone` once `#[graphql_scalar]` macro
//       supports generics.
#[graphql_scalar(
    description = "Combined date and time (with time zone) in [RFC 3339][0] \
                   format.\
                   \n\n\
                   Represents a description of an exact instant on the \
                   time-line (such as the instant that a user account was \
                   created).\
                   \n\n\
                   [`DateTime` scalar][1] compliant.\
                   \n\n\
                   See also [`chrono::DateTime`][2]` and \
                   [`chrono::FixedOffset`][3]` for details.\
                   \n\n\
                   [0]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6\n\
                   [1]: https://graphql-scalars.dev/docs/scalars/date-time\n\
                   [2]: https://docs.rs/chrono/*/chrono/struct.DateTime.html\n\",
                   [3]: https://docs.rs/chrono/*/chrono/offset/struct.FixedOffset.html",
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/date-time"
)]
impl<S: ScalarValue> GraphQLScalar for DateTime<UtcOffset> {
    fn resolve(&self) -> Value {
        Value::scalar(
            self.with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::AutoSi, true),
        )
    }

    fn from_input_value(v: &InputValue) -> Result<Self, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                Self::parse_from_rfc3339(s).map_err(|e| format!("Invalid `DateTime`: {}", e))
            })
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        if let ScalarToken::String(s) = value {
            Ok(S::from(s.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

#[cfg(test)]
mod date_test {
    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::Date;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            ("1996-12-19", Date::from_ymd(1996, 12, 19)),
            ("1564-01-30", Date::from_ymd(1564, 01, 30)),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = Date::from_input_value(&input);

            assert!(
                parsed.is_ok(),
                "failed to parse `{}`: {}",
                raw,
                parsed.unwrap_err(),
            );
            assert_eq!(parsed.unwrap(), expected, "input: {}", raw);
        }
    }

    #[test]
    fn fails_on_invalid_input() {
        for input in [
            graphql_input_value!("1996-13-19"),
            graphql_input_value!("1564-01-61"),
            graphql_input_value!("2021-11-31"),
            graphql_input_value!("11-31"),
            graphql_input_value!("2021-11"),
            graphql_input_value!("2021"),
            graphql_input_value!("31"),
            graphql_input_value!("i'm not even a date"),
            graphql_input_value!(2.32),
            graphql_input_value!(1),
            graphql_input_value!(null),
            graphql_input_value!(false),
        ] {
            let input: InputValue = input;
            let parsed = Date::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {:?}", input);
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                Date::from_ymd(1996, 12, 19),
                graphql_input_value!("1996-12-19"),
            ),
            (
                Date::from_ymd(1564, 01, 30),
                graphql_input_value!("1564-01-30"),
            ),
            (
                Date::from_ymd(2020, 01, 01),
                graphql_input_value!("2020-01-01"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {}", val);
        }
    }
}

#[cfg(test)]
mod local_time_test {
    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::LocalTime;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            ("14:23:43", LocalTime::from_hms(14, 23, 43)),
            ("14:00:00", LocalTime::from_hms(14, 00, 00)),
            ("14:00", LocalTime::from_hms(14, 00, 00)),
            ("14:32", LocalTime::from_hms(14, 32, 00)),
            ("14:00:00.000", LocalTime::from_hms(14, 00, 00)),
            ("14:23:43.345", LocalTime::from_hms_milli(14, 23, 43, 345)),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = LocalTime::from_input_value(&input);

            assert!(
                parsed.is_ok(),
                "failed to parse `{}`: {}",
                raw,
                parsed.unwrap_err(),
            );
            assert_eq!(parsed.unwrap(), expected, "input: {}", raw);
        }
    }

    #[test]
    fn fails_on_invalid_input() {
        for input in [
            graphql_input_value!("12"),
            graphql_input_value!("12:"),
            graphql_input_value!("56:34:22"),
            graphql_input_value!("23:78:43"),
            graphql_input_value!("23:78:"),
            graphql_input_value!("23:18:99"),
            graphql_input_value!("23:18:22."),
            graphql_input_value!("22.03"),
            graphql_input_value!("24:00"),
            graphql_input_value!("24:00:00"),
            graphql_input_value!("24:00:00.000"),
            graphql_input_value!("i'm not even a time"),
            graphql_input_value!(2.32),
            graphql_input_value!(1),
            graphql_input_value!(null),
            graphql_input_value!(false),
        ] {
            let input: InputValue = input;
            let parsed = LocalTime::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {:?}", input);
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                LocalTime::from_hms_micro(1, 2, 3, 4005),
                graphql_input_value!("01:02:03.004"),
            ),
            (
                LocalTime::from_hms(0, 0, 0),
                graphql_input_value!("00:00:00"),
            ),
            (
                LocalTime::from_hms(12, 0, 0),
                graphql_input_value!("12:00:00"),
            ),
            (
                LocalTime::from_hms(1, 2, 3),
                graphql_input_value!("01:02:03"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {}", val);
        }
    }
}

#[cfg(test)]
mod local_date_time_test {
    use chrono::naive::{NaiveDate, NaiveTime};

    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::LocalDateTime;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            (
                "1996-12-19 14:23:43",
                LocalDateTime::new(
                    NaiveDate::from_ymd(1996, 12, 19),
                    NaiveTime::from_hms(14, 23, 43),
                ),
            ),
            (
                "1564-01-30 14:00:00",
                LocalDateTime::new(
                    NaiveDate::from_ymd(1564, 1, 30),
                    NaiveTime::from_hms(14, 00, 00),
                ),
            ),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = LocalDateTime::from_input_value(&input);

            assert!(
                parsed.is_ok(),
                "failed to parse `{}`: {}",
                raw,
                parsed.unwrap_err(),
            );
            assert_eq!(parsed.unwrap(), expected, "input: {}", raw);
        }
    }

    #[test]
    fn fails_on_invalid_input() {
        for input in [
            graphql_input_value!("12"),
            graphql_input_value!("12:"),
            graphql_input_value!("56:34:22"),
            graphql_input_value!("56:34:22.000"),
            graphql_input_value!("1996-12-19T14:23:43"),
            graphql_input_value!("1996-12-19 14:23:43Z"),
            graphql_input_value!("1996-12-19 14:23:43.543"),
            graphql_input_value!("1996-12-19 14:23"),
            graphql_input_value!("1996-12-19 14:23:"),
            graphql_input_value!("1996-12-19 23:78:43"),
            graphql_input_value!("1996-12-19 23:18:99"),
            graphql_input_value!("1996-12-19 24:00:00"),
            graphql_input_value!("1996-12-19 99:02:13"),
            graphql_input_value!("i'm not even a datetime"),
            graphql_input_value!(2.32),
            graphql_input_value!(1),
            graphql_input_value!(null),
            graphql_input_value!(false),
        ] {
            let input: InputValue = input;
            let parsed = LocalDateTime::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {:?}", input);
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                LocalDateTime::new(
                    NaiveDate::from_ymd(1996, 12, 19),
                    NaiveTime::from_hms(0, 0, 0),
                ),
                graphql_input_value!("1996-12-19 00:00:00"),
            ),
            (
                LocalDateTime::new(
                    NaiveDate::from_ymd(1564, 1, 30),
                    NaiveTime::from_hms(14, 0, 0),
                ),
                graphql_input_value!("1564-01-30 14:00:00"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {}", val);
        }
    }
}

#[cfg(test)]
mod date_time_test {
    use chrono::naive::{NaiveDate, NaiveDateTime, NaiveTime};

    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::{DateTime, UtcOffset};

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            (
                "2014-11-28T21:00:09+09:00",
                DateTime::<UtcOffset>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd(2014, 11, 28),
                        NaiveTime::from_hms(12, 0, 9),
                    ),
                    UtcOffset::east(9 * 3600),
                ),
            ),
            (
                "2014-11-28T21:00:09Z",
                DateTime::<UtcOffset>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd(2014, 11, 28),
                        NaiveTime::from_hms(21, 0, 9),
                    ),
                    UtcOffset::east(0),
                ),
            ),
            (
                "2014-11-28T21:00:09+00:00",
                DateTime::<UtcOffset>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd(2014, 11, 28),
                        NaiveTime::from_hms(21, 0, 9),
                    ),
                    UtcOffset::east(0),
                ),
            ),
            (
                "2014-11-28T21:00:09.05+09:00",
                DateTime::<UtcOffset>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd(2014, 11, 28),
                        NaiveTime::from_hms_milli(12, 0, 9, 50),
                    ),
                    UtcOffset::east(0),
                ),
            ),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = DateTime::<UtcOffset>::from_input_value(&input);

            assert!(
                parsed.is_ok(),
                "failed to parse `{}`: {}",
                raw,
                parsed.unwrap_err(),
            );
            assert_eq!(parsed.unwrap(), expected, "input: {}", raw);
        }
    }

    #[test]
    fn fails_on_invalid_input() {
        for input in [
            graphql_input_value!("12"),
            graphql_input_value!("12:"),
            graphql_input_value!("56:34:22"),
            graphql_input_value!("56:34:22.000"),
            graphql_input_value!("1996-12-1914:23:43"),
            graphql_input_value!("1996-12-19 14:23:43Z"),
            graphql_input_value!("1996-12-19T14:23:43"),
            graphql_input_value!("1996-12-19T14:23:43ZZ"),
            graphql_input_value!("1996-12-19T14:23:43.543"),
            graphql_input_value!("1996-12-19T14:23"),
            graphql_input_value!("1996-12-19T14:23:1"),
            graphql_input_value!("1996-12-19T14:23:"),
            graphql_input_value!("1996-12-19T23:78:43Z"),
            graphql_input_value!("1996-12-19T23:18:99Z"),
            graphql_input_value!("1996-12-19T24:00:00Z"),
            graphql_input_value!("1996-12-19T99:02:13Z"),
            graphql_input_value!("1996-12-19T99:02:13Z"),
            graphql_input_value!("1996-12-19T12:02:13+4444444"),
            graphql_input_value!("i'm not even a datetime"),
            graphql_input_value!(2.32),
            graphql_input_value!(1),
            graphql_input_value!(null),
            graphql_input_value!(false),
        ] {
            let input: InputValue = input;
            let parsed = DateTime::<UtcOffset>::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {:?}", input);
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                DateTime::<UtcOffset>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd(1996, 12, 19),
                        NaiveTime::from_hms(0, 0, 0),
                    ),
                    UtcOffset::east(0),
                ),
                graphql_input_value!("1996-12-19T00:00:00Z"),
            ),
            (
                DateTime::<UtcOffset>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd(1564, 1, 30),
                        NaiveTime::from_hms_milli(5, 0, 0, 123),
                    ),
                    UtcOffset::east(9 * 3600),
                ),
                graphql_input_value!("1564-01-30T05:00:00.123Z"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {}", val);
        }
    }
}

#[cfg(test)]
mod integration_test {
    use crate::{
        execute, graphql_object, graphql_value, graphql_vars,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
    };

    use super::{Date, DateTime, LocalDateTime, LocalTime, UtcOffset};

    #[tokio::test]
    async fn serializes() {
        struct Root;

        #[graphql_object]
        impl Root {
            fn date() -> Date {
                Date::from_ymd(2015, 3, 14)
            }

            fn local_time() -> LocalTime {
                LocalTime::from_hms(16, 7, 8)
            }

            fn local_date_time() -> LocalDateTime {
                LocalDateTime::new(Date::from_ymd(2016, 7, 8), LocalTime::from_hms(9, 10, 11))
            }

            fn date_time() -> DateTime<UtcOffset> {
                DateTime::<UtcOffset>::from_utc(
                    LocalDateTime::new(
                        Date::from_ymd(1996, 12, 20),
                        LocalTime::from_hms(0, 39, 57),
                    ),
                    UtcOffset::west(8 * 3600),
                )
            }
        }

        const DOC: &str = r#"{
            date
            localTime
            localDateTime
            dateTime,
        }"#;

        let schema = RootNode::new(
            Root,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({
                    "date": "2015-03-14",
                    "localTime": "16:07:08",
                    "localDateTime": "2016-07-08 09:10:11",
                    "dateTime": "1996-12-20T00:39:57Z",
                }),
                vec![],
            )),
        );
    }
}
