//! GraphQL support for [`chrono`] crate types.
//!
//! # Supported types
//!
//! | Rust type                         | Format                | GraphQL scalar      |
//! |-----------------------------------|-----------------------|---------------------|
//! | [`NaiveDate`]                     | `yyyy-MM-dd`          | [`Date`][s1]        |
//! | [`NaiveTime`]                     | `HH:mm[:ss[.SSS]]`    | [`LocalTime`][s2]   |
//! | [`NaiveDateTime`]                 | `yyyy-MM-dd HH:mm:ss` | `LocalDateTime`     |
//! | [`DateTime`]`<`[`FixedOffset`]`>` | [RFC 3339] string     | [`DateTime`][s4]    |
//! | [`FixedOffset`]                   | `±hh:mm`              | [`UtcOffset`][s5]   |
//!
//! [`DateTime`]: chrono::DateTime
//! [`FixedOffset`]: chrono::FixedOffset
//! [`NaiveDate`]: chrono::NaiveDate
//! [`NaiveDateTime`]: chrono::NaiveDateTime
//! [`NaiveTime`]: chrono::NaiveTime
//! [RFC 3339]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6
//! [s1]: https://graphql-scalars.dev/docs/scalars/date
//! [s2]: https://graphql-scalars.dev/docs/scalars/local-time
//! [s4]: https://graphql-scalars.dev/docs/scalars/date-time
//! [s5]: https://graphql-scalars.dev/docs/scalars/utc-offset

use std::str::FromStr;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

use crate::{
    graphql_scalar,
    parser::{ParseError, ScalarToken, Token},
    value::ParseScalarResult,
    Value,
};

/// Format of a [`Date` scalar][1].
///
/// [1]: https://graphql-scalars.dev/docs/scalars/date
const DATE_FORMAT: &str = "%Y-%m-%d";

#[graphql_scalar(
    description = "Date",
    description = "Date in the proleptic Gregorian calendar (without time \
                   zone).\
                   \n\n\
                   Represents a description of the date (as used for birthdays,
                   for example). It cannot represent an instant on the \
                   time-line.\
                   \n\n\
                   [`Date` scalar][1] compliant.\
                   \n\n\
                   See also [`time::Date`][2] for details.\
                   \n\n\
                   [1]: https://graphql-scalars.dev/docs/scalars/date\n\
                   [2]: https://docs.rs/time/*/time/struct.Date.html",
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/date"
)]
impl<S> GraphQLScalar for NaiveDate
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.format(DATE_FORMAT).to_string())
    }

    fn from_input_value(v: &InputValue) -> Result<NaiveDate, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                Self::parse_from_str(s, "%Y-%m-%d").map_err(|e| format!("Invalid `Date`: {}", e))
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
    description = "LocalTime",
    description = "Clock time within a given date (without time zone) in \
                   `HH:mm[:ss[.SSS]]` format.\
                   \n\n\
                   All minutes are assumed to have exactly 60 seconds; no \
                   attempt is made to handle leap seconds (either positive or \
                   negative).\
                   \n\n\
                   [`LocalTime` scalar][1] compliant.\
                   \n\n\
                   See also [`time::Time`][2] for details.\
                   \n\n\
                   [1]: https://graphql-scalars.dev/docs/scalars/local-time\n\
                   [2]: https://docs.rs/time/*/time/struct.Time.html",
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/local-time"
)]
impl<S> GraphQLScalar for NaiveTime
where
    S: ScalarValue,
{
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

    fn from_input_value(v: &InputValue) -> Result<NaiveTime, String> {
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
        if let ScalarToken::String(value) = value {
            Ok(S::from(value.to_owned()))
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

/// Format of a `LocalDateTime` scalar.
const LOCAL_DATE_TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[graphql_scalar(
    description = "LocalDateTime",
    description = "Combined date and time (without time zone) in `yyyy-MM-dd \
                   HH:mm:ss` format.\
                   \n\n\
                   See also [`time::PrimitiveDateTime`][2] for details.\
                   \n\n\
                   [2]: https://docs.rs/time/*/time/struct.PrimitiveDateTime.html"
)]
impl<S> GraphQLScalar for NaiveDateTime
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.format(LOCAL_DATE_TIME_FORMAT).to_string())
    }

    fn from_input_value(v: &InputValue) -> Result<NaiveDateTime, String> {
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

/// Format of a [`DateTime` scalar][1].
///
/// [1]: https://graphql-scalars.dev/docs/scalars/date-time
const DATE_TIME_FORMAT: &str = "%FT%TZ";

#[graphql_scalar(
    name = "DateTime",
    description = "Combined date and time (with time zone) in [RFC 3339][0] \
                   format.\
                   \n\n\
                   Represents a description of an exact instant on the \
                   time-line (such as the instant that a user account was \
                   created).\
                   \n\n\
                   [`DateTime` scalar][1] compliant.\
                   \n\n\
                   See also [`time::OffsetDateTime`][2] for details.\
                   \n\n\
                   [0]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6\n\
                   [1]: https://graphql-scalars.dev/docs/scalars/date-time\n\
                   [2]: https://docs.rs/time/*/time/struct.OffsetDateTime.html",
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/date-time"
)]
impl<S> GraphQLScalar for DateTime<FixedOffset>
where
    S: ScalarValue,
{
    fn resolve(&self) -> Value {
        Value::scalar(self.naive_utc().format(DATE_TIME_FORMAT).to_string())
    }

    fn from_input_value(v: &InputValue) -> Result<DateTime<FixedOffset>, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                DateTime::parse_from_rfc3339(s).map_err(|e| format!("Invalid `DateTime`: {}", e))
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

#[graphql_scalar(
    name = "UtcOffset",
    description = "Offset from UTC in `±hh:mm` format. See [list of database \
                   time zones][0].\
                   \n\n\
                   [`UtcOffset` scalar][1] compliant.\
                   \n\n\
                   See also [`time::UtcOffset`][2] for details.\
                   \n\n\
                   [0]: https://en.wikipedia.org/wiki/List_of_tz_database_time_zones\n\
                   [1]: https://graphql-scalars.dev/docs/scalars/utc-offset\n\
                   [2]: https://docs.rs/time/*/time/struct.UtcOffset.html",
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/utc-offset"
)]
impl<S: ScalarValue> GraphQLScalar for FixedOffset {
    fn resolve(&self) -> Value {
        let hh = self.local_minus_utc() / 3600;
        let mm = (self.local_minus_utc() % 3600) / 60;

        Value::scalar(format!("{:+}:{}", hh, mm.abs()))
    }

    fn from_input_value(v: &InputValue) -> Result<Self, String> {
        const ERR_PREFIX: &str = "Invalid `UtcOffset`";

        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s: &str| {
                let (hh, mm) = s
                    .get(1..=2)
                    .and_then(|hh| s.get(4..=5).map(|mm| (hh, mm)))
                    .filter(|_| s.chars().count() == 6)
                    .ok_or_else(|| {
                        format!("{}: Expected exactly 6 characters: `±hh:mm`", ERR_PREFIX,)
                    })?;

                let (hh, mm) = u16::from_str(hh)
                    .and_then(|hh| u16::from_str(mm).map(|mm| (hh, mm)))
                    .map_err(|e| format!("{}: {}", ERR_PREFIX, e))?;
                let offset = i32::from(hh * 3600 + mm * 60);

                match (s.chars().next(), s.chars().skip(3).next()) {
                    (Some('+'), Some(':')) => FixedOffset::east_opt(offset),
                    (Some('-'), Some(':')) => FixedOffset::west_opt(offset),
                    _ => return Err(format!("{}: Expected format `±hh:mm`", ERR_PREFIX)),
                }
                .ok_or_else(|| format!("{}: out-of-bound offset", ERR_PREFIX))
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

    use super::NaiveDate;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            ("1996-12-19", NaiveDate::from_ymd(1996, 12, 19)),
            ("1564-01-30", NaiveDate::from_ymd(1564, 01, 30)),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = NaiveDate::from_input_value(&input);

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
            let parsed = NaiveDate::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {:?}", input);
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                NaiveDate::from_ymd(1996, 12, 19),
                graphql_input_value!("1996-12-19"),
            ),
            (
                NaiveDate::from_ymd(1564, 01, 30),
                graphql_input_value!("1564-01-30"),
            ),
            (
                NaiveDate::from_ymd(2020, 01, 01),
                graphql_input_value!("2020-01-01"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {}", val);
        }
    }
}

#[cfg(test)]
mod naive_time_test {
    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::NaiveTime;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            ("14:23:43", NaiveTime::from_hms(14, 23, 43)),
            ("14:00:00", NaiveTime::from_hms(14, 00, 00)),
            ("14:00", NaiveTime::from_hms(14, 00, 00)),
            ("14:32", NaiveTime::from_hms(14, 32, 00)),
            ("14:00:00.000", NaiveTime::from_hms(14, 00, 00)),
            ("14:23:43.345", NaiveTime::from_hms_milli(14, 23, 43, 345)),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = NaiveTime::from_input_value(&input);

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
            let parsed = NaiveTime::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {:?}", input);
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                NaiveTime::from_hms_micro(1, 2, 3, 4005),
                graphql_input_value!("01:02:03.004"),
            ),
            (
                NaiveTime::from_hms(0, 0, 0),
                graphql_input_value!("00:00:00"),
            ),
            (
                NaiveTime::from_hms(12, 0, 0),
                graphql_input_value!("12:00:00"),
            ),
            (
                NaiveTime::from_hms(1, 2, 3),
                graphql_input_value!("01:02:03"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {}", val);
        }
    }
}

#[cfg(test)]
mod naive_date_time_test {
    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::{NaiveDate, NaiveDateTime, NaiveTime};

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            (
                "1996-12-19 14:23:43",
                NaiveDateTime::new(
                    NaiveDate::from_ymd(1996, 12, 19),
                    NaiveTime::from_hms(14, 23, 43),
                ),
            ),
            (
                "1564-01-30 14:00:00",
                NaiveDateTime::new(
                    NaiveDate::from_ymd(1564, 1, 30),
                    NaiveTime::from_hms(14, 00, 00),
                ),
            ),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = NaiveDateTime::from_input_value(&input);

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
            let parsed = NaiveDateTime::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {:?}", input);
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                NaiveDateTime::new(
                    NaiveDate::from_ymd(1996, 12, 19),
                    NaiveTime::from_hms(0, 0, 0),
                ),
                graphql_input_value!("1996-12-19 00:00:00"),
            ),
            (
                NaiveDateTime::new(
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
    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            (
                "2014-11-28T21:00:09+09:00",
                DateTime::<FixedOffset>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd(2014, 11, 28),
                        NaiveTime::from_hms(12, 0, 9),
                    ),
                    FixedOffset::east(9 * 3600),
                ),
            ),
            (
                "2014-11-28T21:00:09Z",
                DateTime::<FixedOffset>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd(2014, 11, 28),
                        NaiveTime::from_hms(21, 0, 9),
                    ),
                    FixedOffset::east(0),
                ),
            ),
            (
                "2014-11-28T21:00:09+00:00",
                DateTime::<FixedOffset>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd(2014, 11, 28),
                        NaiveTime::from_hms(21, 0, 9),
                    ),
                    FixedOffset::east(0),
                ),
            ),
            (
                "2014-11-28T21:00:09.05+09:00",
                DateTime::<FixedOffset>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd(2014, 11, 28),
                        NaiveTime::from_hms_milli(12, 0, 9, 50),
                    ),
                    FixedOffset::east(0),
                ),
            ),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = DateTime::from_input_value(&input);

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
            let parsed = DateTime::<FixedOffset>::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {:?}", input);
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                DateTime::<FixedOffset>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd(1996, 12, 19),
                        NaiveTime::from_hms(0, 0, 0),
                    ),
                    FixedOffset::east(0),
                ),
                graphql_input_value!("1996-12-19T00:00:00Z"),
            ),
            (
                DateTime::<FixedOffset>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd(1564, 1, 30),
                        NaiveTime::from_hms(5, 0, 0),
                    ),
                    FixedOffset::east(9 * 3600),
                ),
                graphql_input_value!("1564-01-30T05:00:00Z"),
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

    use super::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};

    #[tokio::test]
    async fn serializes() {
        struct Root;

        #[graphql_object]
        impl Root {
            fn date() -> NaiveDate {
                NaiveDate::from_ymd(2015, 3, 14)
            }

            fn local_time() -> NaiveTime {
                NaiveTime::from_hms(16, 7, 8)
            }

            fn local_date_time() -> NaiveDateTime {
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2016, 7, 8),
                    NaiveTime::from_hms(9, 10, 11),
                )
            }

            fn date_time() -> DateTime<FixedOffset> {
                DateTime::<FixedOffset>::from_utc(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd(1996, 12, 20),
                        NaiveTime::from_hms(0, 39, 57),
                    ),
                    FixedOffset::west(8 * 3600),
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
