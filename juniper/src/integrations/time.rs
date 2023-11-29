//! GraphQL support for [`time`] crate types.
//!
//! # Supported types
//!
//! | Rust type             | Format                | GraphQL scalar      |
//! |-----------------------|-----------------------|---------------------|
//! | [`Date`]              | `yyyy-MM-dd`          | [`Date`][s1]        |
//! | [`Time`]              | `HH:mm[:ss[.SSS]]`    | [`LocalTime`][s2]   |
//! | [`PrimitiveDateTime`] | `yyyy-MM-dd HH:mm:ss` | `LocalDateTime`     |
//! | [`OffsetDateTime`]    | [RFC 3339] string     | [`DateTime`][s4]    |
//! | [`UtcOffset`]         | `±hh:mm`              | [`UtcOffset`][s5]   |
//!
//! [`Date`]: time::Date
//! [`OffsetDateTime`]: time::OffsetDateTime
//! [`PrimitiveDateTime`]: time::PrimitiveDateTime
//! [`Time`]: time::Time
//! [`UtcOffset`]: time::UtcOffset
//! [RFC 3339]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6
//! [s1]: https://graphql-scalars.dev/docs/scalars/date
//! [s2]: https://graphql-scalars.dev/docs/scalars/local-time
//! [s4]: https://graphql-scalars.dev/docs/scalars/date-time
//! [s5]: https://graphql-scalars.dev/docs/scalars/utc-offset

use time::{
    format_description::{well_known::Rfc3339, FormatItem},
    macros::format_description,
};

use crate::{graphql_scalar, InputValue, ScalarValue, Value};

/// Date in the proleptic Gregorian calendar (without time zone).
///
/// Represents a description of the date (as used for birthdays, for example).
/// It cannot represent an instant on the time-line.
///
/// [`Date` scalar][1] compliant.
///
/// See also [`time::Date`][2] for details.
///
/// [1]: https://graphql-scalars.dev/docs/scalars/date
/// [2]: https://docs.rs/time/*/time/struct.Date.html
#[graphql_scalar(
    with = date,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/date",
)]
pub type Date = time::Date;

mod date {
    use super::*;

    /// Format of a [`Date` scalar][1].
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/date
    const FORMAT: &[FormatItem<'_>] = format_description!("[year]-[month]-[day]");

    pub(super) fn to_output<S: ScalarValue>(v: &Date) -> Value<S> {
        Value::scalar(
            v.format(FORMAT)
                .unwrap_or_else(|e| panic!("Failed to format `Date`: {e}")),
        )
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Date, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| Date::parse(s, FORMAT).map_err(|e| format!("Invalid `Date`: {e}")))
    }
}

/// Clock time within a given date (without time zone) in `HH:mm[:ss[.SSS]]`
/// format.
///
/// All minutes are assumed to have exactly 60 seconds; no attempt is made to
/// handle leap seconds (either positive or negative).
///
/// [`LocalTime` scalar][1] compliant.
///
/// See also [`time::Time`][2] for details.
///
/// [1]: https://graphql-scalars.dev/docs/scalars/local-time
/// [2]: https://docs.rs/time/*/time/struct.Time.html
#[graphql_scalar(with = local_time, parse_token(String))]
pub type LocalTime = time::Time;

mod local_time {
    use super::*;

    /// Full format of a [`LocalTime` scalar][1].
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/local-time
    const FORMAT: &[FormatItem<'_>] =
        format_description!("[hour]:[minute]:[second].[subsecond digits:3]");

    /// Format of a [`LocalTime` scalar][1] without milliseconds.
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/local-time
    const FORMAT_NO_MILLIS: &[FormatItem<'_>] = format_description!("[hour]:[minute]:[second]");

    /// Format of a [`LocalTime` scalar][1] without seconds.
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/local-time
    const FORMAT_NO_SECS: &[FormatItem<'_>] = format_description!("[hour]:[minute]");

    pub(super) fn to_output<S: ScalarValue>(v: &LocalTime) -> Value<S> {
        Value::scalar(
            if v.millisecond() == 0 {
                v.format(FORMAT_NO_MILLIS)
            } else {
                v.format(FORMAT)
            }
            .unwrap_or_else(|e| panic!("Failed to format `LocalTime`: {e}")),
        )
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<LocalTime, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| {
                // First, try to parse the most used format.
                // At the end, try to parse the full format for the parsing
                // error to be most informative.
                LocalTime::parse(s, FORMAT_NO_MILLIS)
                    .or_else(|_| LocalTime::parse(s, FORMAT_NO_SECS))
                    .or_else(|_| LocalTime::parse(s, FORMAT))
                    .map_err(|e| format!("Invalid `LocalTime`: {e}"))
            })
    }
}

/// Combined date and time (without time zone) in `yyyy-MM-dd HH:mm:ss` format.
///
/// See also [`time::PrimitiveDateTime`][2] for details.
///
/// [2]: https://docs.rs/time/*/time/struct.PrimitiveDateTime.html
#[graphql_scalar(with = local_date_time, parse_token(String))]
pub type LocalDateTime = time::PrimitiveDateTime;

mod local_date_time {
    use super::*;

    /// Format of a [`LocalDateTime`] scalar.
    const FORMAT: &[FormatItem<'_>] =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

    pub(super) fn to_output<S: ScalarValue>(v: &LocalDateTime) -> Value<S> {
        Value::scalar(
            v.format(FORMAT)
                .unwrap_or_else(|e| panic!("Failed to format `LocalDateTime`: {e}")),
        )
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<LocalDateTime, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| {
                LocalDateTime::parse(s, FORMAT).map_err(|e| format!("Invalid `LocalDateTime`: {e}"))
            })
    }
}

/// Combined date and time (with time zone) in [RFC 3339][0] format.
///
/// Represents a description of an exact instant on the time-line (such as the
/// instant that a user account was created).
///
/// [`DateTime` scalar][1] compliant.
///
/// See also [`time::OffsetDateTime`][2] for details.
///
/// [0]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6
/// [1]: https://graphql-scalars.dev/docs/scalars/date-time
/// [2]: https://docs.rs/time/*/time/struct.OffsetDateTime.html
#[graphql_scalar(
    with = date_time,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/date-time",
)]
pub type DateTime = time::OffsetDateTime;

mod date_time {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &DateTime) -> Value<S> {
        Value::scalar(
            v.to_offset(UtcOffset::UTC)
                .format(&Rfc3339)
                .unwrap_or_else(|e| panic!("Failed to format `DateTime`: {e}")),
        )
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<DateTime, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| {
                DateTime::parse(s, &Rfc3339).map_err(|e| format!("Invalid `DateTime`: {e}"))
            })
            .map(|dt| dt.to_offset(UtcOffset::UTC))
    }
}

/// Format of a [`UtcOffset` scalar][1].
///
/// [1]: https://graphql-scalars.dev/docs/scalars/utc-offset
const UTC_OFFSET_FORMAT: &[FormatItem<'_>] =
    format_description!("[offset_hour sign:mandatory]:[offset_minute]");

/// Offset from UTC in `±hh:mm` format. See [list of database time zones][0].
///
/// [`UtcOffset` scalar][1] compliant.
///
/// See also [`time::UtcOffset`][2] for details.
///
/// [0]: https://en.wikipedia.org/wiki/List_of_tz_database_time_zones
/// [1]: https://graphql-scalars.dev/docs/scalars/utc-offset
/// [2]: https://docs.rs/time/*/time/struct.UtcOffset.html
#[graphql_scalar(
    with = utc_offset,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/utc-offset",
)]
pub type UtcOffset = time::UtcOffset;

mod utc_offset {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &UtcOffset) -> Value<S> {
        Value::scalar(
            v.format(UTC_OFFSET_FORMAT)
                .unwrap_or_else(|e| panic!("Failed to format `UtcOffset`: {e}")),
        )
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<UtcOffset, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| {
                UtcOffset::parse(s, UTC_OFFSET_FORMAT)
                    .map_err(|e| format!("Invalid `UtcOffset`: {e}"))
            })
    }
}

#[cfg(test)]
mod date_test {
    use time::macros::date;

    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::Date;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            ("1996-12-19", date!(1996 - 12 - 19)),
            ("1564-01-30", date!(1564 - 01 - 30)),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = Date::from_input_value(&input);

            assert!(
                parsed.is_ok(),
                "failed to parse `{raw}`: {:?}",
                parsed.unwrap_err(),
            );
            assert_eq!(parsed.unwrap(), expected, "input: {raw}");
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

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (date!(1996 - 12 - 19), graphql_input_value!("1996-12-19")),
            (date!(1564 - 01 - 30), graphql_input_value!("1564-01-30")),
            (date!(2020 - W 01 - 3), graphql_input_value!("2020-01-01")),
            (date!(2020 - 001), graphql_input_value!("2020-01-01")),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}

#[cfg(test)]
mod local_time_test {
    use time::macros::time;

    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::LocalTime;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            ("14:23:43", time!(14:23:43)),
            ("14:00:00", time!(14:00)),
            ("14:00", time!(14:00)),
            ("14:32", time!(14:32:00)),
            ("14:00:00.000", time!(14:00)),
            ("14:23:43.345", time!(14:23:43.345)),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = LocalTime::from_input_value(&input);

            assert!(
                parsed.is_ok(),
                "failed to parse `{raw}`: {:?}",
                parsed.unwrap_err(),
            );
            assert_eq!(parsed.unwrap(), expected, "input: {raw}");
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
            graphql_input_value!("23:18:22.4351"),
            graphql_input_value!("23:18:22."),
            graphql_input_value!("23:18:22.3"),
            graphql_input_value!("23:18:22.03"),
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

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (time!(1:02:03.004_005), graphql_input_value!("01:02:03.004")),
            (time!(0:00), graphql_input_value!("00:00:00")),
            (time!(12:00 pm), graphql_input_value!("12:00:00")),
            (time!(1:02:03), graphql_input_value!("01:02:03")),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}

#[cfg(test)]
mod local_date_time_test {
    use time::macros::datetime;

    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::LocalDateTime;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            ("1996-12-19 14:23:43", datetime!(1996-12-19 14:23:43)),
            ("1564-01-30 14:00:00", datetime!(1564-01-30 14:00)),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = LocalDateTime::from_input_value(&input);

            assert!(
                parsed.is_ok(),
                "failed to parse `{raw}`: {:?}",
                parsed.unwrap_err(),
            );
            assert_eq!(parsed.unwrap(), expected, "input: {raw}");
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
            graphql_input_value!("1996-12-19T14:23:43"),
            graphql_input_value!("1996-12-19 14:23:43Z"),
            graphql_input_value!("1996-12-19 14:23:43.543"),
            graphql_input_value!("1996-12-19 14:23"),
            graphql_input_value!("1996-12-19 14:23:1"),
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

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                datetime!(1996-12-19 12:00 am),
                graphql_input_value!("1996-12-19 00:00:00"),
            ),
            (
                datetime!(1564-01-30 14:00),
                graphql_input_value!("1564-01-30 14:00:00"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}

#[cfg(test)]
mod date_time_test {
    use time::macros::datetime;

    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::DateTime;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            (
                "2014-11-28T21:00:09+09:00",
                datetime!(2014-11-28 21:00:09 +9),
            ),
            ("2014-11-28T21:00:09Z", datetime!(2014-11-28 21:00:09 +0)),
            (
                "2014-11-28T21:00:09+00:00",
                datetime!(2014-11-28 21:00:09 +0),
            ),
            (
                "2014-11-28T21:00:09.05+09:00",
                datetime!(2014-11-28 12:00:09.05 +0),
            ),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = DateTime::from_input_value(&input);

            assert!(
                parsed.is_ok(),
                "failed to parse `{raw}`: {:?}",
                parsed.unwrap_err(),
            );
            assert_eq!(parsed.unwrap(), expected, "input: {raw}");
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
            let parsed = DateTime::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                datetime!(1996-12-19 12:00 am UTC),
                graphql_input_value!("1996-12-19T00:00:00Z"),
            ),
            (
                datetime!(1564-01-30 14:00 +9),
                graphql_input_value!("1564-01-30T05:00:00Z"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}

#[cfg(test)]
mod utc_offset_test {
    use time::macros::offset;

    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::UtcOffset;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            ("+00:00", offset!(+0)),
            ("-00:00", offset!(-0)),
            ("+10:00", offset!(+10)),
            ("-07:30", offset!(-7:30)),
            ("+14:00", offset!(+14)),
            ("-12:00", offset!(-12)),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = UtcOffset::from_input_value(&input);

            assert!(
                parsed.is_ok(),
                "failed to parse `{raw}`: {:?}",
                parsed.unwrap_err(),
            );
            assert_eq!(parsed.unwrap(), expected, "input: {raw}");
        }
    }

    #[test]
    fn fails_on_invalid_input() {
        for input in [
            graphql_input_value!("12"),
            graphql_input_value!("12:"),
            graphql_input_value!("12:00"),
            graphql_input_value!("+12:"),
            graphql_input_value!("+12:0"),
            graphql_input_value!("+12:00:34"),
            graphql_input_value!("+12"),
            graphql_input_value!("-12"),
            graphql_input_value!("-12:"),
            graphql_input_value!("-12:0"),
            graphql_input_value!("-12:00:32"),
            graphql_input_value!("-999:00"),
            graphql_input_value!("+999:00"),
            graphql_input_value!("i'm not even an offset"),
            graphql_input_value!(2.32),
            graphql_input_value!(1),
            graphql_input_value!(null),
            graphql_input_value!(false),
        ] {
            let input: InputValue = input;
            let parsed = UtcOffset::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (offset!(+1), graphql_input_value!("+01:00")),
            (offset!(+0), graphql_input_value!("+00:00")),
            (offset!(-2:30), graphql_input_value!("-02:30")),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}

#[cfg(test)]
mod integration_test {
    use time::macros::{date, datetime, offset, time};

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
                date!(2015 - 03 - 14)
            }

            fn local_time() -> LocalTime {
                time!(16:07:08)
            }

            fn local_date_time() -> LocalDateTime {
                datetime!(2016-07-08 09:10:11)
            }

            fn date_time() -> DateTime {
                datetime!(1996-12-19 16:39:57 -8)
            }

            fn utc_offset() -> UtcOffset {
                offset!(+11:30)
            }
        }

        const DOC: &str = r#"{
            date
            localTime
            localDateTime
            dateTime,
            utcOffset,
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
                    "utcOffset": "+11:30",
                }),
                vec![],
            )),
        );
    }
}
