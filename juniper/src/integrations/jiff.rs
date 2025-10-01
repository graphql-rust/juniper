//! GraphQL support for [`jiff`] crate types.
//!
//! # Supported types
//!
//! | Rust type                              | Format                     | GraphQL scalar        |
//! |----------------------------------------|----------------------------|-----------------------|
//! | [`civil::Date`]                        | `yyyy-MM-dd`               | [`LocalDate`][s1]     |
//! | [`civil::Time`]                        | `HH:mm[:ss[.SSS]]`         | [`LocalTime`][s2]     |
//! | [`civil::DateTime`]                    | `yyyy-MM-ddTHH:mm:ss`      | [`LocalDateTime`][s3] |
//! | [`Timestamp`]                          | [RFC 3339] string          | [`DateTime`][s4]      |
//! | [`Zoned`] [^1]                         | [RFC 9557] string          | `ZonedDateTime`       |
//! | [`tz::TimeZone`] [^1]                  | [IANA] identifier/`±hh:mm` | `TimeZoneOrUtcOffset` |
//! | [`tz::TimeZone`] via [`TimeZone`] [^1] | [IANA] identifier          | [`TimeZone`][s5]      |
//! | [`tz::Offset`]                         | `±hh:mm`                   | [`UtcOffset`][s6]     |
//! | [`Span`]                               | [ISO 8601] duration        | [`Duration`][s7]      |
//!
//! # [`tz::TimeZone`] types
//!
//! [`tz::TimeZone`] values can be either [IANA] identifiers or fixed offsets, corresponding to
//! GraphQL scalars [`TimeZone`][s5] and [`UtcOffset`][s6] accordingly. While a [`UtcOffset`][s6]
//! GraphQL scalar can be serialized from a [`tz::Offset`] directly, the newtype [`TimeZone`]
//! handles serialization to a [`TimeZone`][s5] GraphQL scalar, with implementations [`TryFrom`] and
//! [`Into`] a [`tz::TimeZone`].
//!
//! In addition, a [`tz::TimeZone`] serializes to a `TimeZoneOrUtcOffset` GraphQL scalar, containing
//! either an [IANA] identifier or a fixed offset for clients being able to consume both values.
//!
//! [^1]: For these, crate [`jiff`] must be installed with a feature flag that provides access to
//! the [IANA Time Zone Database][IANA] (e.g. by using the crate's default feature flags).
//! See [`jiff` time zone features][1] for details.
//!
//! [`civil::Date`]: jiff::civil::Date
//! [`civil::DateTime`]: jiff::civil::DateTime
//! [`civil::Time`]: jiff::civil::Time
//! [`Span`]: jiff::Span
//! [`Timestamp`]: jiff::Timestamp
//! [`tz::Offset`]: jiff::tz::Offset
//! [`tz::TimeZone`]: jiff::tz::TimeZone
//! [`Zoned`]: jiff::Zoned
//! [IANA]: http://iana.org/time-zones
//! [ISO 8601]: https://en.wikipedia.org/wiki/ISO_8601#Durations
//! [RFC 3339]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6
//! [RFC 9557]: https://datatracker.ietf.org/doc/html/rfc9557#section-4.1
//! [s1]: https://graphql-scalars.dev/docs/scalars/local-date
//! [s2]: https://graphql-scalars.dev/docs/scalars/local-time
//! [s3]: https://graphql-scalars.dev/docs/scalars/local-date-time
//! [s4]: https://graphql-scalars.dev/docs/scalars/date-time
//! [s5]: https://graphql-scalars.dev/docs/scalars/time-zone
//! [s6]: https://graphql-scalars.dev/docs/scalars/utc-offset
//! [s7]: https://graphql-scalars.dev/docs/scalars/duration
//! [1]: https://docs.rs/jiff/latest/jiff/index.html#time-zone-features

use std::str;

use derive_more::with_trait::{Debug, Display, Error, Into};

use crate::{ScalarValue, graphql_scalar};

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
/// [2]: https://docs.rs/jiff/*/jiff/civil/struct.Date.html
#[graphql_scalar]
#[graphql(
    with = local_date,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/local-date",
)]
pub type LocalDate = jiff::civil::Date;

mod local_date {
    use std::fmt::Display;

    use super::LocalDate;

    /// Format of a [`LocalDate` scalar][1].
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/local-date
    const FORMAT: &str = "%Y-%m-%d";

    pub(super) fn to_output(v: &LocalDate) -> impl Display {
        v.strftime(FORMAT)
    }

    pub(super) fn from_input(s: &str) -> Result<LocalDate, Box<str>> {
        LocalDate::strptime(FORMAT, s).map_err(|e| format!("Invalid `LocalDate`: {e}").into())
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
/// [2]: https://docs.rs/jiff/*/jiff/civil/struct.Time.html
#[graphql_scalar]
#[graphql(
    with = local_time,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/local-time",
)]
pub type LocalTime = jiff::civil::Time;

mod local_time {
    use std::fmt::Display;

    use super::LocalTime;

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

    pub(super) fn to_output(v: &LocalTime) -> impl Display {
        if v.subsec_nanosecond() == 0 {
            v.strftime(FORMAT_NO_MILLIS)
        } else {
            v.strftime(FORMAT)
        }
    }

    pub(super) fn from_input(s: &str) -> Result<LocalTime, Box<str>> {
        // First, try to parse the most used format.
        // At the end, try to parse the full format for the parsing error to be most informative.
        LocalTime::strptime(FORMAT_NO_MILLIS, s)
            .or_else(|_| LocalTime::strptime(FORMAT_NO_SECS, s))
            .or_else(|_| LocalTime::strptime(FORMAT, s))
            .map_err(|e| format!("Invalid `LocalTime`: {e}").into())
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
/// [2]: https://docs.rs/jiff/*/jiff/civil/struct.DateTime.html
#[graphql_scalar]
#[graphql(
    with = local_date_time,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/local-date-time",
)]
pub type LocalDateTime = jiff::civil::DateTime;

mod local_date_time {
    use std::fmt::Display;

    use super::LocalDateTime;

    /// Format of a [`LocalDateTime` scalar][1].
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/local-date-time
    const FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

    pub(super) fn to_output(v: &LocalDateTime) -> impl Display {
        v.strftime(FORMAT)
    }

    pub(super) fn from_input(s: &str) -> Result<LocalDateTime, Box<str>> {
        LocalDateTime::strptime(FORMAT, s)
            .map_err(|e| format!("Invalid `LocalDateTime`: {e}").into())
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
/// [2]: https://docs.rs/jiff/*/jiff/struct.Timestamp.html
#[graphql_scalar]
#[graphql(
    with = date_time,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/date-time",
)]
pub type DateTime = jiff::Timestamp;

mod date_time {
    use std::fmt::Display;

    use super::DateTime;

    /// Format of a [`DateTime` scalar][1].
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/date-time
    const FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.fZ";

    pub(super) fn to_output(v: &DateTime) -> impl Display {
        v.strftime(FORMAT)
    }

    pub(super) fn from_input(s: &str) -> Result<DateTime, Box<str>> {
        s.parse()
            .map_err(|e| format!("Invalid `DateTime`: {e}").into())
    }
}

/// Time zone aware instant in time.
///
/// Can be thought of as combination of the following types, all rolled into one:
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
#[graphql_scalar]
#[graphql(
    with = zoned_date_time,
    to_output_with = ScalarValue::from_displayable,
    parse_token(String),
    specified_by_url = "https://datatracker.ietf.org/doc/html/rfc9557#section-4.1",
)]
pub type ZonedDateTime = jiff::Zoned;

mod zoned_date_time {
    use super::ZonedDateTime;

    pub(super) fn from_input(s: &str) -> Result<ZonedDateTime, Box<str>> {
        s.parse()
            .map_err(|e| format!("Invalid `ZonedDateTime`: {e}").into())
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
/// [2]: https://docs.rs/jiff/*/jiff/struct.Span.html
#[graphql_scalar]
#[graphql(
    with = duration,
    to_output_with = ScalarValue::from_displayable,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/duration",
)]
pub type Duration = jiff::Span;

mod duration {
    use super::Duration;

    pub(super) fn from_input(s: &str) -> Result<Duration, Box<str>> {
        s.parse()
            .map_err(|e| format!("Invalid `Duration`: {e}").into())
    }
}

/// Representation of a time zone or UTC offset.
///
/// Can be one of three possible representations:
/// - Identifier from the [IANA Time Zone Database][0].
/// - Fixed offset from UTC (`±hh:mm`).
///
/// May be seen as a combination of both [`TimeZone`][3] and [`UtcOffset` scalars][4].
///
/// See also [`jiff::tz::TimeZone`][2] for details.
///
/// [0]: http://iana.org/time-zones
/// [2]: https://docs.rs/jiff/latest/jiff/tz/struct.TimeZone.html
/// [3]: https://graphql-scalars.dev/docs/scalars/time-zone
/// [4]: https://graphql-scalars.dev/docs/scalars/utc-offset
#[graphql_scalar]
#[graphql(
    with = time_zone_or_utc_offset,
    parse_token(String),
)]
pub type TimeZoneOrUtcOffset = jiff::tz::TimeZone;

mod time_zone_or_utc_offset {
    use std::fmt::Display;

    use super::{TimeZoneOrUtcOffset, TimeZoneParsingError, utc_offset};
    use crate::util::Either;

    /// Format of a [`TimeZoneOrUtcOffset`] scalar.
    const FORMAT: &str = "%:Q";

    pub(super) fn to_output(v: &TimeZoneOrUtcOffset) -> impl Display {
        if let Some(name) = v.iana_name() {
            Either::Left(name)
        } else {
            // If no IANA time zone identifier is available, fall back to displaying the time
            // offset directly (using format `[+-]HH:MM[:SS]` from RFC 9557, e.g. `+05:30`).
            // See: https://github.com/graphql-rust/juniper/pull/1278#discussion_r1719161686
            Either::Right(
                jiff::Zoned::now()
                    .with_time_zone(v.clone())
                    .strftime(FORMAT),
            )
        }
    }

    pub(super) fn from_input(s: &str) -> Result<TimeZoneOrUtcOffset, Box<str>> {
        TimeZoneOrUtcOffset::get(s)
            .map_err(TimeZoneParsingError::InvalidTimeZone)
            .or_else(|_| utc_offset::utc_offset_from_str(s).map(TimeZoneOrUtcOffset::fixed))
            .map_err(|e| format!("Invalid `TimeZoneOrUtcOffset`: {e}").into())
    }
}

/// Error parsing a [`TimeZone`] value.
#[derive(Clone, Debug, Display, Error)]
pub enum TimeZoneParsingError {
    /// Identifier cannot not be parsed by the [`jiff::tz::TimeZone::get()`] method.
    InvalidTimeZone(jiff::Error),

    /// GraphQL scalar [`TimeZone`] requires `tz::TimeZone` with IANA name.
    #[display("missing IANA name")]
    MissingIanaName(
        #[debug(ignore)]
        #[error(not(source))]
        jiff::tz::TimeZone,
    ),
}

/// Representation of a time zone from the [IANA Time Zone Database][0].
///
/// A set of rules for determining the civil time, via an offset from UTC, in a particular
/// geographic region. In many cases, the offset in a particular time zone can vary over the course
/// of a year through transitions into and out of daylight saving time.
///
/// [`TimeZone` scalar][1] compliant.
///
/// See also [`jiff::tz::TimeZone`][2] for details.
///
/// [0]: http://iana.org/time-zones
/// [1]: https://graphql-scalars.dev/docs/scalars/time-zone
/// [2]: https://docs.rs/jiff/latest/jiff/tz/struct.TimeZone.html
#[graphql_scalar]
#[graphql(
    with = time_zone,
    to_output_with = ScalarValue::from_displayable,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/time-zone",
)]
#[derive(Clone, Debug, Display, Eq, Into, PartialEq)]
#[display("{}", _0.iana_name().expect("failed to display `TimeZone`: no IANA name"))]
pub struct TimeZone(jiff::tz::TimeZone);

impl TryFrom<jiff::tz::TimeZone> for TimeZone {
    type Error = TimeZoneParsingError;

    fn try_from(value: jiff::tz::TimeZone) -> Result<Self, Self::Error> {
        if value.iana_name().is_none() {
            return Err(TimeZoneParsingError::MissingIanaName(value));
        }
        Ok(Self(value))
    }
}

impl str::FromStr for TimeZone {
    type Err = TimeZoneParsingError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value =
            jiff::tz::TimeZone::get(value).map_err(TimeZoneParsingError::InvalidTimeZone)?;
        value.try_into()
    }
}

mod time_zone {
    use super::TimeZone;

    pub(super) fn from_input(s: &str) -> Result<TimeZone, Box<str>> {
        s.parse()
            .map_err(|e| format!("Invalid `TimeZone`: {e}").into())
    }
}

/// Representation of a fixed time zone offset.
///
/// [`UtcOffset` scalar][1] compliant.
///
/// See also [`jiff::tz::Offset`][2] for details.
///
/// [1]: https://graphql-scalars.dev/docs/scalars/utc-offset
/// [2]: https://docs.rs/jiff/latest/jiff/tz/struct.Offset.html
#[graphql_scalar]
#[graphql(
    with = utc_offset,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/utc-offset",
)]
pub type UtcOffset = jiff::tz::Offset;

mod utc_offset {
    use std::fmt::{self, Display};

    use jiff::fmt::{StdFmtWrite, strtime::BrokenDownTime};

    use super::UtcOffset;

    /// Format of a [`UtcOffset` scalar][1].
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/utc-offset
    const FORMAT: &str = "%:z";

    pub(super) fn utc_offset_from_str(value: &str) -> Result<jiff::tz::Offset, jiff::Error> {
        let tm = BrokenDownTime::parse(FORMAT, value)?;
        let offset = tm
            .offset()
            .expect("successful %:z parsing guarantees offset");
        Ok(offset)
    }

    pub(super) fn to_output(v: &UtcOffset) -> impl Display {
        struct LazyFmt(BrokenDownTime);

        impl Display for LazyFmt {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0
                    .format(FORMAT, StdFmtWrite(f))
                    .map_err(|_| fmt::Error)
            }
        }

        LazyFmt(BrokenDownTime::from(
            &jiff::Zoned::now().with_time_zone(jiff::tz::TimeZone::fixed(*v)),
        ))
    }

    pub(super) fn from_input(s: &str) -> Result<UtcOffset, Box<str>> {
        utc_offset_from_str(s).map_err(|e| format!("Invalid `UtcOffset`: {e}").into())
    }
}

#[cfg(test)]
mod local_date_test {
    use crate::{FromInputValue as _, InputValue, ToInputValue as _, graphql};

    use super::LocalDate;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            ("1996-12-19", LocalDate::constant(1996, 12, 19)),
            ("1564-01-30", LocalDate::constant(1564, 01, 30)),
        ] {
            let input: InputValue = graphql::input_value!((raw));
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
            graphql::input_value!("1996-13-19"),
            graphql::input_value!("1564-01-61"),
            graphql::input_value!("2021-11-31"),
            graphql::input_value!("11-31"),
            graphql::input_value!("2021-11"),
            graphql::input_value!("2021"),
            graphql::input_value!("31"),
            graphql::input_value!("i'm not even a date"),
            graphql::input_value!(2.32),
            graphql::input_value!(1),
            graphql::input_value!(null),
            graphql::input_value!(false),
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
                graphql::input_value!("1996-12-19"),
            ),
            (
                LocalDate::constant(1564, 01, 30),
                graphql::input_value!("1564-01-30"),
            ),
            (
                LocalDate::constant(2020, 01, 01),
                graphql::input_value!("2020-01-01"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}

#[cfg(test)]
mod local_time_test {
    use crate::{FromInputValue as _, InputValue, ToInputValue as _, graphql};

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
            let input: InputValue = graphql::input_value!((raw));
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
            graphql::input_value!("12"),
            graphql::input_value!("12:"),
            graphql::input_value!("56:34:22"),
            graphql::input_value!("23:78:43"),
            graphql::input_value!("23:78:"),
            graphql::input_value!("23:18:99"),
            graphql::input_value!("23:18:22."),
            graphql::input_value!("22.03"),
            graphql::input_value!("24:00"),
            graphql::input_value!("24:00:00"),
            graphql::input_value!("24:00:00.000"),
            graphql::input_value!("i'm not even a time"),
            graphql::input_value!(2.32),
            graphql::input_value!(1),
            graphql::input_value!(null),
            graphql::input_value!(false),
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
                graphql::input_value!("01:02:03.004"),
            ),
            (
                LocalTime::constant(0, 0, 0, 0),
                graphql::input_value!("00:00:00"),
            ),
            (
                LocalTime::constant(12, 0, 0, 0),
                graphql::input_value!("12:00:00"),
            ),
            (
                LocalTime::constant(1, 2, 3, 0),
                graphql::input_value!("01:02:03"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}

#[cfg(test)]
mod local_date_time_test {
    use crate::{FromInputValue as _, InputValue, ToInputValue as _, graphql};

    use super::LocalDateTime;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            (
                "1996-12-19T14:23:43",
                LocalDateTime::constant(1996, 12, 19, 14, 23, 43, 0),
            ),
            (
                "1564-01-30T14:00:00",
                LocalDateTime::constant(1564, 1, 30, 14, 00, 00, 0),
            ),
        ] {
            let input: InputValue = graphql::input_value!((raw));
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
            graphql::input_value!("12"),
            graphql::input_value!("12:"),
            graphql::input_value!("56:34:22"),
            graphql::input_value!("56:34:22.000"),
            graphql::input_value!("1996-12-1914:23:43"),
            graphql::input_value!("1996-12-19 14:23:43"),
            graphql::input_value!("1996-12-19Q14:23:43"),
            graphql::input_value!("1996-12-19T14:23:43Z"),
            graphql::input_value!("1996-12-19T14:23:43.543"),
            graphql::input_value!("1996-12-19T14:23"),
            graphql::input_value!("1996-12-19T14:23:"),
            graphql::input_value!("1996-12-19T23:78:43"),
            graphql::input_value!("1996-12-19T23:18:99"),
            graphql::input_value!("1996-12-19T24:00:00"),
            graphql::input_value!("1996-12-19T99:02:13"),
            graphql::input_value!("i'm not even a datetime"),
            graphql::input_value!(2.32),
            graphql::input_value!(1),
            graphql::input_value!(null),
            graphql::input_value!(false),
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
                graphql::input_value!("1996-12-19T00:00:00"),
            ),
            (
                LocalDateTime::constant(1564, 1, 30, 14, 0, 0, 0),
                graphql::input_value!("1564-01-30T14:00:00"),
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

    use crate::{FromInputValue as _, InputValue, ToInputValue as _, graphql};

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
            let input: InputValue = graphql::input_value!((raw));
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
            graphql::input_value!("12"),
            graphql::input_value!("12:"),
            graphql::input_value!("56:34:22"),
            graphql::input_value!("56:34:22.000"),
            graphql::input_value!("1996-12-1914:23:43"),
            graphql::input_value!("1996-12-19 14:23:43"),
            graphql::input_value!("1996-12-19Q14:23:43Z"),
            graphql::input_value!("1996-12-19T14:23:43"),
            graphql::input_value!("1996-12-19T14:23:43ZZ"),
            graphql::input_value!("1996-12-19T14:23:43.543"),
            graphql::input_value!("1996-12-19T14:23"),
            graphql::input_value!("1996-12-19T14:23:1"),
            graphql::input_value!("1996-12-19T14:23:"),
            graphql::input_value!("1996-12-19T23:78:43Z"),
            graphql::input_value!("1996-12-19T23:18:99Z"),
            graphql::input_value!("1996-12-19T24:00:00Z"),
            graphql::input_value!("1996-12-19T99:02:13Z"),
            graphql::input_value!("1996-12-19T99:02:13Z"),
            graphql::input_value!("1996-12-19T12:02:13+4444444"),
            graphql::input_value!("i'm not even a datetime"),
            graphql::input_value!(2.32),
            graphql::input_value!(1),
            graphql::input_value!(null),
            graphql::input_value!(false),
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
                graphql::input_value!("1996-12-19T00:00:00Z"),
            ),
            (
                civil::DateTime::constant(1564, 1, 30, 5, 0, 0, 123_000_000)
                    .to_zoned(TimeZone::UTC)
                    .unwrap()
                    .timestamp(),
                graphql::input_value!("1564-01-30T05:00:00.123Z"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}

#[cfg(test)]
mod zoned_date_time_test {
    use jiff::{civil, tz, tz::TimeZone};

    use crate::{FromInputValue as _, InputValue, ToInputValue as _, graphql};

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
            let input: InputValue = graphql::input_value!((raw));
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
            graphql::input_value!("12"),
            graphql::input_value!("12:"),
            graphql::input_value!("56:34:22"),
            graphql::input_value!("56:34:22.000"),
            graphql::input_value!("1996-12-1914:23:43"),
            graphql::input_value!("1996-12-19Q14:23:43Z"),
            graphql::input_value!("1996-12-19T14:23:43"),
            graphql::input_value!("1996-12-19T14:23:43ZZ"),
            graphql::input_value!("1996-12-19T14:23:43.543"),
            graphql::input_value!("1996-12-19T14:23"),
            graphql::input_value!("1996-12-19T14:23:1"),
            graphql::input_value!("1996-12-19T14:23:"),
            graphql::input_value!("1996-12-19T23:78:43Z"),
            graphql::input_value!("1996-12-19T23:18:99Z"),
            graphql::input_value!("1996-12-19T24:00:00Z"),
            graphql::input_value!("1996-12-19T99:02:13Z"),
            graphql::input_value!("1996-12-19T99:02:13Z"),
            graphql::input_value!("1996-12-19T12:02:13+4444444"),
            graphql::input_value!("i'm not even a datetime"),
            graphql::input_value!("2014-11-28T21:00:09Z"),
            graphql::input_value!("2014-11-28T21:00:09+09:00"),
            graphql::input_value!("2014-11-28T21:00:09+09:00[InvTZ]"),
            graphql::input_value!(2.32),
            graphql::input_value!(1),
            graphql::input_value!(null),
            graphql::input_value!(false),
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
                graphql::input_value!("1996-12-19T00:00:00-05:00[America/New_York]"),
            ),
            (
                civil::DateTime::constant(1964, 7, 30, 5, 0, 0, 123_000_000)
                    .to_zoned(TimeZone::get("America/New_York").unwrap())
                    .unwrap(),
                graphql::input_value!("1964-07-30T05:00:00.123-04:00[America/New_York]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::get("GMT+0").unwrap())
                    .unwrap(),
                graphql::input_value!("2014-11-28T21:00:09+00:00[GMT+0]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::get("Etc/GMT+3").unwrap())
                    .unwrap(),
                graphql::input_value!("2014-11-28T21:00:09-03:00[Etc/GMT+3]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::get("UTC").unwrap())
                    .unwrap(),
                graphql::input_value!("2014-11-28T21:00:09+00:00[UTC]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::UTC)
                    .unwrap(),
                graphql::input_value!("2014-11-28T21:00:09+00:00[UTC]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::fixed(tz::offset(0)))
                    .unwrap(),
                graphql::input_value!("2014-11-28T21:00:09+00:00[UTC]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::fixed(tz::offset(2)))
                    .unwrap(),
                graphql::input_value!("2014-11-28T21:00:09+02:00[+02:00]"),
            ),
            (
                civil::DateTime::constant(2014, 11, 28, 21, 0, 9, 0)
                    .to_zoned(TimeZone::fixed(tz::offset(-11)))
                    .unwrap(),
                graphql::input_value!("2014-11-28T21:00:09-11:00[-11:00]"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}

#[cfg(test)]
mod duration_test {
    use jiff::ToSpan as _;

    use crate::{FromInputValue as _, InputValue, ToInputValue as _, graphql};

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
            let input: InputValue = graphql::input_value!((raw));
            let parsed = Duration::from_input_value(&input).map(Duration::fieldwise);

            assert!(
                parsed.is_ok(),
                "failed to parse `{raw}`: {:?}",
                parsed.unwrap_err(),
            );
            assert_eq!(parsed.unwrap(), expected, "input: {raw}");
        }
    }

    #[test]
    fn parses_jiff_friendly_input() {
        for (raw, expected) in [
            ("40d", 40.days()),
            ("40 days", 40.days()),
            ("1y1d", 1.year().days(1)),
            ("1yr 1d", 1.year().days(1)),
            ("3d4h59m", 3.days().hours(4).minutes(59)),
            ("3 days, 4 hours, 59 minutes", 3.days().hours(4).minutes(59)),
            ("3d 4h 59m", 3.days().hours(4).minutes(59)),
            ("2h30m", 2.hours().minutes(30)),
            ("2h 30m", 2.hours().minutes(30)),
            ("1mo", 1.month()),
            ("1w", 1.week()),
            ("1 week", 1.week()),
            ("1w4d", 1.week().days(4)),
            ("1 wk 4 days", 1.week().days(4)),
            ("1m", 1.minute()),
            ("0.0021s", 2.milliseconds().microseconds(100)),
            ("0s", 0.seconds()),
            ("0d", 0.seconds()),
            ("0 days", 0.seconds()),
            (
                "1y1mo1d1h1m1.1s",
                1.year()
                    .months(1)
                    .days(1)
                    .hours(1)
                    .minutes(1)
                    .seconds(1)
                    .milliseconds(100),
            ),
            (
                "1yr 1mo 1day 1hr 1min 1.1sec",
                1.year()
                    .months(1)
                    .days(1)
                    .hours(1)
                    .minutes(1)
                    .seconds(1)
                    .milliseconds(100),
            ),
            (
                "1 year, 1 month, 1 day, 1 hour, 1 minute 1.1 seconds",
                1.year()
                    .months(1)
                    .days(1)
                    .hours(1)
                    .minutes(1)
                    .seconds(1)
                    .milliseconds(100),
            ),
            (
                "1 year, 1 month, 1 day, 01:01:01.1",
                1.year()
                    .months(1)
                    .days(1)
                    .hours(1)
                    .minutes(1)
                    .seconds(1)
                    .milliseconds(100),
            ),
        ] {
            let input: InputValue = graphql::input_value!((raw));
            let parsed = Duration::from_input_value(&input).map(Duration::fieldwise);

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
            graphql::input_value!("12"),
            graphql::input_value!("12S"),
            graphql::input_value!("P0"),
            graphql::input_value!("PT"),
            graphql::input_value!("PTS"),
            graphql::input_value!("1996-12-19"),
            graphql::input_value!("1996-12-19T14:23:43"),
            graphql::input_value!("1996-12-19T14:23:43Z"),
            graphql::input_value!("i'm not even a duration"),
            graphql::input_value!(2.32),
            graphql::input_value!(1),
            graphql::input_value!(null),
            graphql::input_value!(false),
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
                graphql::input_value!("P1Y1M1DT1H1M1.1S"),
            ),
            ((-5).days(), graphql::input_value!("-P5D")),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
        }
    }
}

#[cfg(test)]
mod time_zone_or_utc_offset_test {
    use jiff::tz;

    use crate::{FromInputValue as _, InputValue, ToInputValue as _, graphql};

    use super::TimeZoneOrUtcOffset;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            (
                "Europe/London",
                TimeZoneOrUtcOffset::get("Europe/London").unwrap(),
            ),
            ("Etc/GMT-3", TimeZoneOrUtcOffset::get("Etc/GMT-3").unwrap()),
            (
                "etc/gmt+11",
                TimeZoneOrUtcOffset::get("Etc/GMT+11").unwrap(),
            ),
            ("factory", TimeZoneOrUtcOffset::get("Factory").unwrap()),
            ("zULU", TimeZoneOrUtcOffset::get("Zulu").unwrap()),
            ("UTC", TimeZoneOrUtcOffset::get("UTC").unwrap()),
            (
                "+00:00",
                TimeZoneOrUtcOffset::try_from(tz::TimeZone::fixed(tz::offset(0))).unwrap(),
            ),
            (
                "+03:00",
                TimeZoneOrUtcOffset::try_from(tz::TimeZone::fixed(tz::offset(3))).unwrap(),
            ),
            (
                "-09:00",
                TimeZoneOrUtcOffset::try_from(tz::TimeZone::fixed(tz::offset(-9))).unwrap(),
            ),
        ] {
            let input: InputValue = graphql::input_value!((raw));
            let parsed = TimeZoneOrUtcOffset::from_input_value(&input);

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
            graphql::input_value!("Abc/Xyz"),
            graphql::input_value!("8086"),
            graphql::input_value!("AbcXyz"),
            graphql::input_value!("Z"),
            graphql::input_value!("i'm not even a time zone"),
            graphql::input_value!(2.32),
            graphql::input_value!(1),
            graphql::input_value!(null),
            graphql::input_value!(false),
        ] {
            let input: InputValue = input;
            let parsed = TimeZoneOrUtcOffset::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                TimeZoneOrUtcOffset::get("Europe/London").unwrap(),
                graphql::input_value!("Europe/London"),
            ),
            (
                TimeZoneOrUtcOffset::get("Etc/GMT-3").unwrap(),
                graphql::input_value!("Etc/GMT-3"),
            ),
            (
                TimeZoneOrUtcOffset::get("etc/gmt+11").unwrap(),
                graphql::input_value!("Etc/GMT+11"),
            ),
            (
                TimeZoneOrUtcOffset::get("Factory").unwrap(),
                graphql::input_value!("Factory"),
            ),
            (
                TimeZoneOrUtcOffset::get("zulu").unwrap(),
                graphql::input_value!("Zulu"),
            ),
            (
                TimeZoneOrUtcOffset::fixed(tz::offset(0)),
                graphql::input_value!("UTC"),
            ),
            (
                TimeZoneOrUtcOffset::get("UTC").unwrap(),
                graphql::input_value!("UTC"),
            ),
            (TimeZoneOrUtcOffset::UTC, graphql::input_value!("UTC")),
            (
                TimeZoneOrUtcOffset::fixed(tz::offset(2)),
                graphql::input_value!("+02:00"),
            ),
            (
                TimeZoneOrUtcOffset::fixed(tz::offset(-11)),
                graphql::input_value!("-11:00"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val:?}");
        }
    }
}

#[cfg(test)]
mod time_zone_test {
    use jiff::tz;

    use crate::{FromInputValue as _, InputValue, ToInputValue as _, graphql};

    use super::TimeZone;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            (
                "Europe/London",
                TimeZone::try_from(tz::TimeZone::get("Europe/London").unwrap()).unwrap(),
            ),
            (
                "Etc/GMT-3",
                TimeZone::try_from(tz::TimeZone::get("Etc/GMT-3").unwrap()).unwrap(),
            ),
            (
                "etc/gmt+11",
                TimeZone::try_from(tz::TimeZone::get("Etc/GMT+11").unwrap()).unwrap(),
            ),
            (
                "factory",
                TimeZone::try_from(tz::TimeZone::get("Factory").unwrap()).unwrap(),
            ),
            (
                "zULU",
                TimeZone::try_from(tz::TimeZone::get("Zulu").unwrap()).unwrap(),
            ),
            (
                "UTC",
                TimeZone::try_from(tz::TimeZone::get("UTC").unwrap()).unwrap(),
            ),
        ] {
            let input: InputValue = graphql::input_value!((raw));
            let parsed = TimeZone::from_input_value(&input);

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
            graphql::input_value!("Abc/Xyz"),
            graphql::input_value!("8086"),
            graphql::input_value!("AbcXyz"),
            graphql::input_value!("-02:00"),
            graphql::input_value!("+11:00"),
            graphql::input_value!("Z"),
            graphql::input_value!("i'm not even a time zone"),
            graphql::input_value!(2.32),
            graphql::input_value!(1),
            graphql::input_value!(null),
            graphql::input_value!(false),
        ] {
            let input: InputValue = input;
            let parsed = TimeZone::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                TimeZone::try_from(tz::TimeZone::get("Europe/London").unwrap()).unwrap(),
                graphql::input_value!("Europe/London"),
            ),
            (
                TimeZone::try_from(tz::TimeZone::get("Etc/GMT-3").unwrap()).unwrap(),
                graphql::input_value!("Etc/GMT-3"),
            ),
            (
                TimeZone::try_from(tz::TimeZone::get("etc/gmt+11").unwrap()).unwrap(),
                graphql::input_value!("Etc/GMT+11"),
            ),
            (
                TimeZone::try_from(tz::TimeZone::get("Factory").unwrap()).unwrap(),
                graphql::input_value!("Factory"),
            ),
            (
                TimeZone::try_from(tz::TimeZone::get("zulu").unwrap()).unwrap(),
                graphql::input_value!("Zulu"),
            ),
            (
                TimeZone::try_from(tz::TimeZone::fixed(tz::offset(0))).unwrap(),
                graphql::input_value!("UTC"),
            ),
            (
                TimeZone::try_from(tz::TimeZone::get("UTC").unwrap()).unwrap(),
                graphql::input_value!("UTC"),
            ),
            (
                TimeZone::try_from(tz::TimeZone::UTC).unwrap(),
                graphql::input_value!("UTC"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val:?}");
        }
    }
}

#[cfg(test)]
mod utc_offset_test {
    use jiff::tz;

    use crate::{FromInputValue as _, InputValue, ToInputValue as _, graphql};

    use super::UtcOffset;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            ("+00:00", tz::offset(0)),
            ("+03:00", tz::offset(3)),
            ("-09:00", tz::offset(-9)),
        ] {
            let input: InputValue = graphql::input_value!((raw));
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
            graphql::input_value!("Europe/London"),
            graphql::input_value!("Abc/Xyz"),
            graphql::input_value!("8086"),
            graphql::input_value!("AbcXyz"),
            graphql::input_value!("Z"),
            graphql::input_value!("i'm not even a time zone"),
            graphql::input_value!(2.32),
            graphql::input_value!(1),
            graphql::input_value!(null),
            graphql::input_value!(false),
        ] {
            let input: InputValue = input;
            let parsed = UtcOffset::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (tz::offset(0), graphql::input_value!("+00:00")),
            (tz::offset(2), graphql::input_value!("+02:00")),
            (tz::offset(-11), graphql::input_value!("-11:00")),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val:?}");
        }
    }
}

#[cfg(test)]
mod integration_test {
    use jiff::{ToSpan as _, civil, tz};

    use crate::{
        execute, graphql, graphql_object,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
    };

    use super::{
        DateTime, Duration, LocalDate, LocalDateTime, LocalTime, TimeZone, UtcOffset, ZonedDateTime,
    };

    #[tokio::test]
    async fn serializes() {
        struct Root;

        #[graphql_object]
        impl Root {
            fn local_date() -> LocalDate {
                LocalDate::constant(2015, 3, 14)
            }

            fn local_time() -> LocalTime {
                LocalTime::constant(16, 7, 8, 0)
            }

            fn local_date_time() -> LocalDateTime {
                LocalDateTime::constant(2016, 7, 8, 9, 10, 11, 0)
            }

            fn date_time() -> DateTime {
                civil::DateTime::constant(2014, 11, 28, 12, 0, 9, 50_000_000)
                    .to_zoned(tz::TimeZone::UTC)
                    .unwrap()
                    .timestamp()
            }

            fn zoned_date_time() -> ZonedDateTime {
                civil::DateTime::constant(2014, 11, 28, 12, 0, 9, 50_000_000)
                    .to_zoned(tz::TimeZone::get("America/New_York").unwrap())
                    .unwrap()
            }

            fn time_zone() -> TimeZone {
                tz::TimeZone::get("Asia/Tokyo").unwrap().try_into().unwrap()
            }

            fn utc_offset() -> UtcOffset {
                tz::offset(10)
            }

            fn duration() -> Duration {
                1.year()
                    .months(1)
                    .days(1)
                    .hours(1)
                    .minutes(1)
                    .seconds(1)
                    .milliseconds(100)
            }
        }

        const DOC: &str = r#"{
            localDate
            localTime
            localDateTime
            dateTime,
            zonedDateTime,
            timeZone,
            utcOffset,
            duration,
        }"#;

        let schema = RootNode::new(
            Root,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );

        assert_eq!(
            execute(DOC, None, &schema, &graphql::vars! {}, &()).await,
            Ok((
                graphql::value!({
                    "localDate": "2015-03-14",
                    "localTime": "16:07:08",
                    "localDateTime": "2016-07-08T09:10:11",
                    "dateTime": "2014-11-28T12:00:09.05Z",
                    "zonedDateTime": "2014-11-28T12:00:09.05-05:00[America/New_York]",
                    "timeZone": "Asia/Tokyo",
                    "utcOffset": "+10:00",
                    "duration": "P1Y1M1DT1H1M1.1S",
                }),
                vec![],
            )),
        );
    }
}
