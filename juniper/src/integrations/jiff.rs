//! GraphQL support for [`jiff`] crate types.
//!
//! # Supported types
//!
//! | Rust type           | Format                | GraphQL scalar        |
//! |---------------------|-----------------------|-----------------------|
//! | [`civil::Date`]     | `yyyy-MM-dd`          | [`LocalDate`][s1]     |
//! | [`civil::Time`]     | `HH:mm[:ss[.SSS]]`    | [`LocalTime`][s2]     |
//! | [`civil::DateTime`] | `yyyy-MM-ddTHH:mm:ss` | [`LocalDateTime`][s3] |
//! | [`Timestamp`]       | [RFC 3339] string     | [`DateTime`][s4]      |
//! | [`Zoned`][^1]       | [RFC 9557] string     | `ZonedDateTime`       |
//! | [`Span`]            | [ISO 8601] duration   | [`Duration`][s5]      |
//!
//! [^1]: For [`Zoned`], feature flag `jiff-tz` must be enabled and crate [`jiff`] must be installed
//! with a feature flag that provides access to the Time Zone Database (e.g. by using the crate's
//! default feature flags). See [`jiff` time zone features][tz] for details.
//!
//! [`civil::Date`]: jiff::civil::Date
//! [`civil::DateTime`]: jiff::civil::DateTime
//! [`civil::Time`]: jiff::civil::Time
//! [`Span`]: jiff::Span
//! [`Timestamp`]: jiff::Timestamp
//! [`Zoned`]: jiff::Zoned
//! [ISO 8601]: https://en.wikipedia.org/wiki/ISO_8601#Durations
//! [RFC 3339]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6
//! [RFC 9557]: https://datatracker.ietf.org/doc/html/rfc9557#section-4.1
//! [s1]: https://graphql-scalars.dev/docs/scalars/local-date
//! [s2]: https://graphql-scalars.dev/docs/scalars/local-time
//! [s3]: https://graphql-scalars.dev/docs/scalars/local-date-time
//! [s4]: https://graphql-scalars.dev/docs/scalars/date-time
//! [s5]: https://graphql-scalars.dev/docs/scalars/duration
//! [tz]: https://docs.rs/jiff/latest/jiff/index.html#time-zone-features

use crate::{graphql_scalar, InputValue, ScalarValue, Value};

/// Representation of a civil date in the Gregorian calendar.
///
/// Corresponds to a triple of year, month and day. Every value is guaranteed to be a valid
/// Gregorian calendar date. For example, both `2023-02-29` and `2023-11-31` are invalid and cannot
/// be represented.
///
/// [`LocalDate` scalar][1] compliant.
///
/// See also [`jiff::civil::Date`][2] for details.
///
/// [1]: https://graphql-scalars.dev/docs/scalars/local-date
/// [2]: https://docs.rs/jiff/latest/jiff/civil/struct.Date.html
#[graphql_scalar(
    with = local_date,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/local-date",
)]
pub type LocalDate = jiff::civil::Date;

mod local_date {
    use super::*;

    /// Format of a [`LocalDate` scalar][1].
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/local-date
    const FORMAT: &str = "%Y-%m-%d";

    pub(super) fn to_output<S>(v: &LocalDate) -> Value<S>
    where
        S: ScalarValue,
    {
        Value::scalar(v.strftime(FORMAT).to_string())
    }

    pub(super) fn from_input<S>(v: &InputValue<S>) -> Result<LocalDate, String>
    where
        S: ScalarValue,
    {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| {
                LocalDate::strptime(FORMAT, s).map_err(|e| format!("Invalid `LocalDate`: {e}"))
            })
    }
}

/// Representation of a civil "wall clock" time.
///
/// Conceptually, corresponds to the typical hours and minutes that you might see on a clock. This
/// type also contains the second and fractional subsecond (to nanosecond precision) associated with
/// a time.
///
/// [`LocalTime` scalar][1] compliant.
///
/// See also [`jiff::civil::Time`][2] for details.
///
/// [1]: https://graphql-scalars.dev/docs/scalars/local-time
/// [2]: https://docs.rs/jiff/latest/jiff/civil/struct.Time.html
#[graphql_scalar(
    with = local_time,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/local-time",
)]
pub type LocalTime = jiff::civil::Time;

mod local_time {
    use super::*;

    /// Full format of a [`LocalTime` scalar][1].
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/local-time
    const FORMAT: &str = "%H:%M:%S%.3f";

    /// Format of a [`LocalTime` scalar][1] without milliseconds.
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/local-time
    const FORMAT_NO_MILLIS: &str = "%H:%M:%S";

    /// Format of a [`LocalTime` scalar][1] without seconds.
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/local-time
    const FORMAT_NO_SECS: &str = "%H:%M";

    pub(super) fn to_output<S>(v: &LocalTime) -> Value<S>
    where
        S: ScalarValue,
    {
        Value::scalar(
            if v.subsec_nanosecond() == 0 {
                v.strftime(FORMAT_NO_MILLIS)
            } else {
                v.strftime(FORMAT)
            }
            .to_string(),
        )
    }

    pub(super) fn from_input<S>(v: &InputValue<S>) -> Result<LocalTime, String>
    where
        S: ScalarValue,
    {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| {
                // First, try to parse the most used format.
                // At the end, try to parse the full format for the parsing
                // error to be most informative.
                LocalTime::strptime(FORMAT_NO_MILLIS, s)
                    .or_else(|_| LocalTime::strptime(FORMAT_NO_SECS, s))
                    .or_else(|_| LocalTime::strptime(FORMAT, s))
                    .map_err(|e| format!("Invalid `LocalTime`: {e}"))
            })
    }
}

/// Representation of a civil datetime in the Gregorian calendar.
///
/// Corresponds to a pair of a `LocalDate` and a `LocalTime`. That is, a datetime contains a year,
/// month, day, hour, minute, second and the fractional number of nanoseconds.
///
/// Value is guaranteed to contain a valid date and time. For example, neither `2023-02-29T00:00:00`
/// nor `2015-06-30T23:59:60` are valid.
///
/// [`LocalDateTime` scalar][1] compliant.
///
/// See also [`jiff::civil::DateTime`][2] for details.
///
/// [1]: https://graphql-scalars.dev/docs/scalars/local-date-time
/// [2]: https://docs.rs/jiff/latest/jiff/civil/struct.DateTime.html
#[graphql_scalar(
    with = local_date_time,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/local-date-time",
)]
pub type LocalDateTime = jiff::civil::DateTime;

mod local_date_time {
    use super::*;

    /// Format of a [`LocalDateTime` scalar][1].
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/local-date-time
    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub(super) fn to_output<S>(v: &LocalDateTime) -> Value<S>
    where
        S: ScalarValue,
    {
        Value::scalar(v.strftime(FORMAT).to_string())
    }

    pub(super) fn from_input<S>(v: &InputValue<S>) -> Result<LocalDateTime, String>
    where
        S: ScalarValue,
    {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| {
                LocalDateTime::strptime(FORMAT, s)
                    .map_err(|e| format!("Invalid `LocalDateTime`: {e}"))
            })
    }
}

/// Instant in time represented as the number of nanoseconds since the Unix epoch.
///
/// Always in UTC.
///
/// [`DateTime` scalar][1] compliant.
///
/// See also [`jiff::Timestamp`][2] for details.
///
/// [1]: https://graphql-scalars.dev/docs/scalars/date-time
/// [2]: https://docs.rs/jiff/latest/jiff/struct.Timestamp.html
#[graphql_scalar(
    with = date_time,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/date-time",
)]
pub type DateTime = jiff::Timestamp;

mod date_time {
    use std::str::FromStr as _;

    use super::*;

    /// Format of a [`DateTime` scalar][1].
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/date-time
    const FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.fZ";

    pub(super) fn to_output<S>(v: &DateTime) -> Value<S>
    where
        S: ScalarValue,
    {
        Value::scalar(v.strftime(FORMAT).to_string())
    }

    pub(super) fn from_input<S>(v: &InputValue<S>) -> Result<DateTime, String>
    where
        S: ScalarValue,
    {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| DateTime::from_str(s).map_err(|e| format!("Invalid `DateTime`: {e}")))
    }
}

/// Time zone aware instant in time.
///
/// Can be thought of as combination of the following types, all rolled into one:
///
/// - [`Timestamp`][3] for indicating precise instant in time.
/// - [`DateTime`][4] for indicating "civil" calendar date and clock time.
/// - [`TimeZone`][5] for indicating how to apply time zone transitions while performing arithmetic.
///
/// [RFC 9557][1] compliant.
///
/// See also [`jiff::Zoned`][2] for details.
///
/// [1]: https://datatracker.ietf.org/doc/html/rfc9557#section-4.1
/// [2]: https://docs.rs/jiff/latest/jiff/struct.Zoned.html
/// [3]: https://docs.rs/jiff/latest/jiff/struct.Timestamp.html
/// [4]: https://docs.rs/jiff/latest/jiff/civil/struct.DateTime.html
/// [5]: https://docs.rs/jiff/latest/jiff/tz/struct.TimeZone.html
#[cfg(feature = "jiff-tz")]
#[graphql_scalar(
    with = zoned_date_time,
    parse_token(String),
)]
pub type ZonedDateTime = jiff::Zoned;

#[cfg(feature = "jiff-tz")]
mod zoned_date_time {
    use std::str::FromStr as _;

    use super::*;

    pub(super) fn to_output<S>(v: &ZonedDateTime) -> Value<S>
    where
        S: ScalarValue,
    {
        Value::scalar(v.to_string())
    }

    pub(super) fn from_input<S>(v: &InputValue<S>) -> Result<ZonedDateTime, String>
    where
        S: ScalarValue,
    {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| {
                ZonedDateTime::from_str(s).map_err(|e| format!("Invalid `ZonedDateTime`: {e}"))
            })
    }
}

/// Span of time represented via a mixture of calendar and clock units.
///
/// Represents a duration of time in units of years, months, weeks, days, hours, minutes, seconds,
/// milliseconds, microseconds and nanoseconds.
///
/// [`Duration` scalar][1] compliant.
///
/// See also [`jiff::Span`][2] for details.
///
/// [1]: https://graphql-scalars.dev/docs/scalars/duration
/// [2]: https://docs.rs/jiff/latest/jiff/struct.Span.html
#[graphql_scalar(
    with = duration,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/duration",
)]
pub type Duration = jiff::Span;

mod duration {
    use std::str::FromStr as _;

    use super::*;

    pub(super) fn to_output<S>(v: &Duration) -> Value<S>
    where
        S: ScalarValue,
    {
        Value::scalar(v.to_string())
    }

    pub(super) fn from_input<S>(v: &InputValue<S>) -> Result<Duration, String>
    where
        S: ScalarValue,
    {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| Duration::from_str(s).map_err(|e| format!("Invalid `Duration`: {e}")))
    }
}

#[cfg(test)]
mod local_date_test {
    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::LocalDate;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            ("1996-12-19", LocalDate::constant(1996, 12, 19)),
            ("1564-01-30", LocalDate::constant(1564, 01, 30)),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = LocalDate::from_input_value(&input);

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
            let parsed = LocalDate::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                LocalDate::constant(1996, 12, 19),
                graphql_input_value!("1996-12-19"),
            ),
            (
                LocalDate::constant(1564, 01, 30),
                graphql_input_value!("1564-01-30"),
            ),
            (
                LocalDate::constant(2020, 01, 01),
                graphql_input_value!("2020-01-01"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
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
            ("14:23:43", LocalTime::constant(14, 23, 43, 000_000_000)),
            ("14:00:00", LocalTime::constant(14, 00, 00, 000_000_000)),
            ("14:00", LocalTime::constant(14, 00, 00, 000_000_000)),
            ("14:32", LocalTime::constant(14, 32, 00, 000_000_000)),
            ("14:00:00.000", LocalTime::constant(14, 00, 00, 000_000_000)),
            ("14:23:43.345", LocalTime::constant(14, 23, 43, 345_000_000)),
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

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                LocalTime::constant(1, 2, 3, 4_005_000),
                graphql_input_value!("01:02:03.004"),
            ),
            (
                LocalTime::constant(0, 0, 0, 0),
                graphql_input_value!("00:00:00"),
            ),
            (
                LocalTime::constant(12, 0, 0, 0),
                graphql_input_value!("12:00:00"),
            ),
            (
                LocalTime::constant(1, 2, 3, 0),
                graphql_input_value!("01:02:03"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}

#[cfg(test)]
mod local_date_time_test {
    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::LocalDateTime;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            (
                "1996-12-19 14:23:43",
                LocalDateTime::constant(1996, 12, 19, 14, 23, 43, 0),
            ),
            (
                "1564-01-30 14:00:00",
                LocalDateTime::constant(1564, 1, 30, 14, 00, 00, 0),
            ),
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

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                LocalDateTime::constant(1996, 12, 19, 0, 0, 0, 0),
                graphql_input_value!("1996-12-19 00:00:00"),
            ),
            (
                LocalDateTime::constant(1564, 1, 30, 14, 0, 0, 0),
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
    use jiff::{civil, tz::TimeZone};

    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::DateTime;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            (
                "2014-11-28T21:00:09+09:00",
                civil::DateTime::constant(2014, 11, 28, 12, 0, 9, 0)
                    .to_zoned(TimeZone::UTC)
                    .unwrap()
                    .timestamp(),
            ),
            (
                "2014-11-28T21:00:09Z",
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::UTC)
                    .unwrap()
                    .timestamp(),
            ),
            (
                "2014-11-28 21:00:09z",
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::UTC)
                    .unwrap()
                    .timestamp(),
            ),
            (
                "2014-11-28T21:00:09+00:00",
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::UTC)
                    .unwrap()
                    .timestamp(),
            ),
            (
                "2014-11-28T21:00:09.05+09:00",
                civil::DateTime::constant(2014, 11, 28, 12, 0, 9, 50_000_000)
                    .to_zoned(TimeZone::UTC)
                    .unwrap()
                    .timestamp(),
            ),
            (
                "2014-11-28 21:00:09.05+09:00",
                civil::DateTime::constant(2014, 11, 28, 12, 0, 9, 50_000_000)
                    .to_zoned(TimeZone::UTC)
                    .unwrap()
                    .timestamp(),
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
            graphql_input_value!("1996-12-19Q14:23:43Z"),
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
                civil::DateTime::constant(1996, 12, 19, 0, 0, 0, 0)
                    .to_zoned(TimeZone::UTC)
                    .unwrap()
                    .timestamp(),
                graphql_input_value!("1996-12-19T00:00:00Z"),
            ),
            (
                civil::DateTime::constant(1564, 1, 30, 5, 0, 0, 123_000_000)
                    .to_zoned(TimeZone::UTC)
                    .unwrap()
                    .timestamp(),
                graphql_input_value!("1564-01-30T05:00:00.123Z"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}

#[cfg(feature = "jiff-tz")]
#[cfg(test)]
mod zoned_date_time_test {
    use jiff::{civil, tz, tz::TimeZone};

    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::ZonedDateTime;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            (
                "2014-11-28T21:00:09+09:00[Asia/Tokyo]",
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::get("Asia/Tokyo").unwrap())
                    .unwrap(),
            ),
            (
                "2014-11-28T21:00:09[America/New_York]",
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::get("America/New_York").unwrap())
                    .unwrap(),
            ),
            (
                "2014-11-28 21:00:09[America/New_York]",
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::get("America/New_York").unwrap())
                    .unwrap(),
            ),
            (
                "2014-11-28T21:00:09Z[gmt+0]",
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::get("GMT+0").unwrap())
                    .unwrap(),
            ),
            (
                "2014-11-28T21:00:09+03:00[etc/gmt-3]",
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::get("Etc/GMT-3").unwrap())
                    .unwrap(),
            ),
            (
                "2014-11-28T21:00:09+00:00[UTC]",
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::get("UTC").unwrap())
                    .unwrap(),
            ),
            (
                "2014-11-28T21:00:09+02:00[+02:00]",
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::fixed(tz::offset(2)))
                    .unwrap(),
            ),
            (
                "2014-11-28T21:00:09-11:00[-11:00]",
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::fixed(tz::offset(-11)))
                    .unwrap(),
            ),
            (
                "2014-11-28T21:00:09.05+09:00[Asia/Tokyo]",
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 50_000_000)
                    .to_zoned(TimeZone::get("Asia/Tokyo").unwrap())
                    .unwrap(),
            ),
            (
                "2014-11-28 21:00:09.05+09:00[Asia/Tokyo]",
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 50_000_000)
                    .to_zoned(TimeZone::get("Asia/Tokyo").unwrap())
                    .unwrap(),
            ),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = ZonedDateTime::from_input_value(&input);

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
            graphql_input_value!("1996-12-19Q14:23:43Z"),
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
            graphql_input_value!("2014-11-28T21:00:09Z"),
            graphql_input_value!("2014-11-28T21:00:09+09:00"),
            graphql_input_value!("2014-11-28T21:00:09+09:00[InvTZ]"),
            graphql_input_value!(2.32),
            graphql_input_value!(1),
            graphql_input_value!(null),
            graphql_input_value!(false),
        ] {
            let input: InputValue = input;
            let parsed = ZonedDateTime::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                civil::DateTime::constant(1996, 12, 19, 0, 0, 0, 0)
                    .to_zoned(TimeZone::get("America/New_York").unwrap())
                    .unwrap(),
                graphql_input_value!("1996-12-19T00:00:00-05:00[America/New_York]"),
            ),
            (
                civil::DateTime::constant(1964, 7, 30, 5, 0, 0, 123_000_000)
                    .to_zoned(TimeZone::get("America/New_York").unwrap())
                    .unwrap(),
                graphql_input_value!("1964-07-30T05:00:00.123-04:00[America/New_York]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::get("GMT+0").unwrap())
                    .unwrap(),
                graphql_input_value!("2014-11-28T21:00:09+00:00[GMT+0]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::get("Etc/GMT+3").unwrap())
                    .unwrap(),
                graphql_input_value!("2014-11-28T21:00:09-03:00[Etc/GMT+3]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::get("UTC").unwrap())
                    .unwrap(),
                graphql_input_value!("2014-11-28T21:00:09+00:00[UTC]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::UTC)
                    .unwrap(),
                graphql_input_value!("2014-11-28T21:00:09+00:00[UTC]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::fixed(tz::offset(0)))
                    .unwrap(),
                graphql_input_value!("2014-11-28T21:00:09+00:00[UTC]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::fixed(tz::offset(2)))
                    .unwrap(),
                graphql_input_value!("2014-11-28T21:00:09+02:00[+02:00]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::fixed(tz::offset(-11)))
                    .unwrap(),
                graphql_input_value!("2014-11-28T21:00:09-11:00[-11:00]"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}

#[cfg(test)]
mod duration_test {
    use jiff::ToSpan;

    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::Duration;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            ("P5dT8h1m", 5.days().hours(8).minutes(1)),
            ("-P5d", (-5).days()),
            ("P2M10DT2H30M", 2.months().days(10).hours(2).minutes(30)),
            ("P40D", 40.days()),
            ("P1y1d", 1.year().days(1)),
            ("P3dT4h59m", 3.days().hours(4).minutes(59)),
            ("PT2H30M", 2.hours().minutes(30)),
            ("P1m", 1.month()),
            ("P1w", 1.week()),
            ("P1w4d", 1.week().days(4)),
            ("PT1m", 1.minute()),
            ("PT0.0021s", 2.milliseconds().microseconds(100)),
            ("PT0s", 0.seconds()),
            ("P0d", 0.seconds()),
            (
                "P1y1m1dT1h1m1.1s",
                1.year()
                    .months(1)
                    .days(1)
                    .hours(1)
                    .minutes(1)
                    .seconds(1)
                    .milliseconds(100),
            ),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = Duration::from_input_value(&input);

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
            graphql_input_value!("12S"),
            graphql_input_value!("P0"),
            graphql_input_value!("PT"),
            graphql_input_value!("PTS"),
            graphql_input_value!("56:34:22"),
            graphql_input_value!("1996-12-19"),
            graphql_input_value!("1996-12-19T14:23:43"),
            graphql_input_value!("1996-12-19T14:23:43Z"),
            graphql_input_value!("i'm not even a duration"),
            graphql_input_value!(2.32),
            graphql_input_value!(1),
            graphql_input_value!(null),
            graphql_input_value!(false),
        ] {
            let input: InputValue = input;
            let parsed = Duration::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                1.year()
                    .months(1)
                    .days(1)
                    .hours(1)
                    .minutes(1)
                    .seconds(1)
                    .milliseconds(100),
                graphql_input_value!("P1y1m1dT1h1m1.1s"),
            ),
            ((-5).days(), graphql_input_value!("-P5d")),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}
