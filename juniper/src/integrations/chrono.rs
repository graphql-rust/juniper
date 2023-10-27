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

use std::fmt;

use chrono::{FixedOffset, TimeZone};

use crate::{graphql_scalar, InputValue, ScalarValue, Value};

/// Date in the proleptic Gregorian calendar (without time zone).
///
/// Represents a description of the date (as used for birthdays, for example).
/// It cannot represent an instant on the time-line.
///
/// [`Date` scalar][1] compliant.
///
/// See also [`chrono::NaiveDate`][2] for details.
///
/// [1]: https://graphql-scalars.dev/docs/scalars/date
/// [2]: https://docs.rs/chrono/latest/chrono/naive/struct.NaiveDate.html
#[graphql_scalar(
    with = date,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/date",
)]
pub type Date = chrono::NaiveDate;

mod date {
    use super::*;

    /// Format of a [`Date` scalar][1].
    ///
    /// [1]: https://graphql-scalars.dev/docs/scalars/date
    const FORMAT: &str = "%Y-%m-%d";

    pub(super) fn to_output<S>(v: &Date) -> Value<S>
    where
        S: ScalarValue,
    {
        Value::scalar(v.format(FORMAT).to_string())
    }

    pub(super) fn from_input<S>(v: &InputValue<S>) -> Result<Date, String>
    where
        S: ScalarValue,
    {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| {
                Date::parse_from_str(s, FORMAT).map_err(|e| format!("Invalid `Date`: {e}"))
            })
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
/// See also [`chrono::NaiveTime`][2] for details.
///
/// [1]: https://graphql-scalars.dev/docs/scalars/local-time
/// [2]: https://docs.rs/chrono/latest/chrono/naive/struct.NaiveTime.html
#[graphql_scalar(
    with = local_time,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/local-time",
)]
pub type LocalTime = chrono::NaiveTime;

mod local_time {
    use chrono::Timelike as _;

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
            if v.nanosecond() == 0 {
                v.format(FORMAT_NO_MILLIS)
            } else {
                v.format(FORMAT)
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
                LocalTime::parse_from_str(s, FORMAT_NO_MILLIS)
                    .or_else(|_| LocalTime::parse_from_str(s, FORMAT_NO_SECS))
                    .or_else(|_| LocalTime::parse_from_str(s, FORMAT))
                    .map_err(|e| format!("Invalid `LocalTime`: {e}"))
            })
    }
}

/// Combined date and time (without time zone) in `yyyy-MM-dd HH:mm:ss` format.
///
/// See also [`chrono::NaiveDateTime`][1] for details.
///
/// [1]: https://docs.rs/chrono/latest/chrono/naive/struct.NaiveDateTime.html
#[graphql_scalar(with = local_date_time, parse_token(String))]
pub type LocalDateTime = chrono::NaiveDateTime;

mod local_date_time {
    use super::*;

    /// Format of a `LocalDateTime` scalar.
    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub(super) fn to_output<S>(v: &LocalDateTime) -> Value<S>
    where
        S: ScalarValue,
    {
        Value::scalar(v.format(FORMAT).to_string())
    }

    pub(super) fn from_input<S>(v: &InputValue<S>) -> Result<LocalDateTime, String>
    where
        S: ScalarValue,
    {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| {
                LocalDateTime::parse_from_str(s, FORMAT)
                    .map_err(|e| format!("Invalid `LocalDateTime`: {e}"))
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
/// See also [`chrono::DateTime`][2] for details.
///
/// [0]: https://datatracker.ietf.org/doc/html/rfc3339#section-5
/// [1]: https://graphql-scalars.dev/docs/scalars/date-time
/// [2]: https://docs.rs/chrono/latest/chrono/struct.DateTime.html
#[graphql_scalar(
    with = date_time,
    parse_token(String),
    where(
        Tz: TimeZone + FromFixedOffset,
        Tz::Offset: fmt::Display,
    )
)]
pub type DateTime<Tz> = chrono::DateTime<Tz>;

mod date_time {
    use chrono::{SecondsFormat, Utc};

    use super::*;

    pub(super) fn to_output<S, Tz>(v: &DateTime<Tz>) -> Value<S>
    where
        S: ScalarValue,
        Tz: chrono::TimeZone,
        Tz::Offset: fmt::Display,
    {
        Value::scalar(
            v.with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::AutoSi, true),
        )
    }

    pub(super) fn from_input<S, Tz>(v: &InputValue<S>) -> Result<DateTime<Tz>, String>
    where
        S: ScalarValue,
        Tz: TimeZone + FromFixedOffset,
    {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| {
                DateTime::<FixedOffset>::parse_from_rfc3339(s)
                    .map_err(|e| format!("Invalid `DateTime`: {e}"))
                    .map(FromFixedOffset::from_fixed_offset)
            })
    }
}

/// Trait allowing to implement a custom [`TimeZone`], which preserves its
/// [`TimeZone`] information when parsed in a [`DateTime`] GraphQL scalar.
///
/// # Example
///
/// Creating a custom [CET] [`TimeZone`] using [`chrono-tz`] crate. This is
/// required because [`chrono-tz`] uses enum to represent all [`TimeZone`]s, so
/// we have no knowledge of the concrete underlying [`TimeZone`] on the type
/// level.
///
/// ```rust
/// # use chrono::{FixedOffset, TimeZone};
/// # use juniper::{
/// #     integrations::chrono::{FromFixedOffset, DateTime},
/// #     graphql_object,
/// # };
/// #
/// #[derive(Clone, Copy)]
/// struct CET;
///
/// impl TimeZone for CET {
///     type Offset = <chrono_tz::Tz as TimeZone>::Offset;
///
///     fn from_offset(_: &Self::Offset) -> Self {
///         CET
///     }
///
///     fn offset_from_local_date(
///         &self,
///         local: &chrono::NaiveDate,
///     ) -> chrono::LocalResult<Self::Offset> {
///         chrono_tz::CET.offset_from_local_date(local)
///     }
///
///     fn offset_from_local_datetime(
///         &self,
///         local: &chrono::NaiveDateTime,
///     ) -> chrono::LocalResult<Self::Offset> {
///         chrono_tz::CET.offset_from_local_datetime(local)
///     }
///
///     fn offset_from_utc_date(&self, utc: &chrono::NaiveDate) -> Self::Offset {
///         chrono_tz::CET.offset_from_utc_date(utc)
///     }
///
///     fn offset_from_utc_datetime(&self, utc: &chrono::NaiveDateTime) -> Self::Offset {
///         chrono_tz::CET.offset_from_utc_datetime(utc)
///     }
/// }
///
/// impl FromFixedOffset for CET {
///     fn from_fixed_offset(dt: DateTime<FixedOffset>) -> DateTime<Self> {
///         dt.with_timezone(&CET)
///     }
/// }
///
/// struct Root;
///
/// #[graphql_object]
/// impl Root {
///     fn pass_date_time(dt: DateTime<CET>) -> DateTime<CET> {
///         dt
///     }
/// }
/// ```
///
/// [`chrono-tz`]: chrono_tz
/// [CET]: https://en.wikipedia.org/wiki/Central_European_Time
pub trait FromFixedOffset: TimeZone {
    /// Converts the given [`DateTime`]`<`[`FixedOffset`]`>` into a
    /// [`DateTime`]`<Self>`.
    fn from_fixed_offset(dt: DateTime<FixedOffset>) -> DateTime<Self>;
}

impl FromFixedOffset for FixedOffset {
    fn from_fixed_offset(dt: DateTime<Self>) -> DateTime<Self> {
        dt
    }
}

impl FromFixedOffset for chrono::Utc {
    fn from_fixed_offset(dt: DateTime<FixedOffset>) -> DateTime<Self> {
        dt.into()
    }
}

#[cfg(feature = "chrono-clock")]
impl FromFixedOffset for chrono::Local {
    fn from_fixed_offset(dt: DateTime<FixedOffset>) -> DateTime<Self> {
        dt.into()
    }
}

#[cfg(feature = "chrono-tz")]
impl FromFixedOffset for chrono_tz::Tz {
    fn from_fixed_offset(dt: DateTime<FixedOffset>) -> DateTime<Self> {
        dt.with_timezone(&chrono_tz::UTC)
    }
}

#[cfg(test)]
mod date_test {
    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::Date;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            ("1996-12-19", Date::from_ymd_opt(1996, 12, 19)),
            ("1564-01-30", Date::from_ymd_opt(1564, 01, 30)),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = Date::from_input_value(&input);

            assert!(
                parsed.is_ok(),
                "failed to parse `{raw}`: {:?}",
                parsed.unwrap_err(),
            );
            assert_eq!(parsed.unwrap(), expected.unwrap(), "input: {raw}");
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
            (
                Date::from_ymd_opt(1996, 12, 19),
                graphql_input_value!("1996-12-19"),
            ),
            (
                Date::from_ymd_opt(1564, 01, 30),
                graphql_input_value!("1564-01-30"),
            ),
            (
                Date::from_ymd_opt(2020, 01, 01),
                graphql_input_value!("2020-01-01"),
            ),
        ] {
            let val = val.unwrap();
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
            ("14:23:43", LocalTime::from_hms_opt(14, 23, 43)),
            ("14:00:00", LocalTime::from_hms_opt(14, 00, 00)),
            ("14:00", LocalTime::from_hms_opt(14, 00, 00)),
            ("14:32", LocalTime::from_hms_opt(14, 32, 00)),
            ("14:00:00.000", LocalTime::from_hms_opt(14, 00, 00)),
            (
                "14:23:43.345",
                LocalTime::from_hms_milli_opt(14, 23, 43, 345),
            ),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = LocalTime::from_input_value(&input);

            assert!(
                parsed.is_ok(),
                "failed to parse `{raw}`: {:?}",
                parsed.unwrap_err(),
            );
            assert_eq!(parsed.unwrap(), expected.unwrap(), "input: {raw}");
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
                LocalTime::from_hms_micro_opt(1, 2, 3, 4005),
                graphql_input_value!("01:02:03.004"),
            ),
            (
                LocalTime::from_hms_opt(0, 0, 0),
                graphql_input_value!("00:00:00"),
            ),
            (
                LocalTime::from_hms_opt(12, 0, 0),
                graphql_input_value!("12:00:00"),
            ),
            (
                LocalTime::from_hms_opt(1, 2, 3),
                graphql_input_value!("01:02:03"),
            ),
        ] {
            let val = val.unwrap();
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
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
                    NaiveDate::from_ymd_opt(1996, 12, 19).unwrap(),
                    NaiveTime::from_hms_opt(14, 23, 43).unwrap(),
                ),
            ),
            (
                "1564-01-30 14:00:00",
                LocalDateTime::new(
                    NaiveDate::from_ymd_opt(1564, 1, 30).unwrap(),
                    NaiveTime::from_hms_opt(14, 00, 00).unwrap(),
                ),
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
                LocalDateTime::new(
                    NaiveDate::from_ymd_opt(1996, 12, 19).unwrap(),
                    NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                ),
                graphql_input_value!("1996-12-19 00:00:00"),
            ),
            (
                LocalDateTime::new(
                    NaiveDate::from_ymd_opt(1564, 1, 30).unwrap(),
                    NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                ),
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
    use chrono::{
        naive::{NaiveDate, NaiveDateTime, NaiveTime},
        FixedOffset,
    };

    use crate::{graphql_input_value, FromInputValue as _, InputValue, ToInputValue as _};

    use super::DateTime;

    #[test]
    fn parses_correct_input() {
        for (raw, expected) in [
            (
                "2014-11-28T21:00:09+09:00",
                DateTime::<FixedOffset>::from_naive_utc_and_offset(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2014, 11, 28).unwrap(),
                        NaiveTime::from_hms_opt(12, 0, 9).unwrap(),
                    ),
                    FixedOffset::east_opt(9 * 3600).unwrap(),
                ),
            ),
            (
                "2014-11-28T21:00:09Z",
                DateTime::<FixedOffset>::from_naive_utc_and_offset(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2014, 11, 28).unwrap(),
                        NaiveTime::from_hms_opt(21, 0, 9).unwrap(),
                    ),
                    FixedOffset::east_opt(0).unwrap(),
                ),
            ),
            (
                "2014-11-28 21:00:09z",
                DateTime::<FixedOffset>::from_naive_utc_and_offset(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2014, 11, 28).unwrap(),
                        NaiveTime::from_hms_opt(21, 0, 9).unwrap(),
                    ),
                    FixedOffset::east_opt(0).unwrap(),
                ),
            ),
            (
                "2014-11-28T21:00:09+00:00",
                DateTime::<FixedOffset>::from_naive_utc_and_offset(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2014, 11, 28).unwrap(),
                        NaiveTime::from_hms_opt(21, 0, 9).unwrap(),
                    ),
                    FixedOffset::east_opt(0).unwrap(),
                ),
            ),
            (
                "2014-11-28T21:00:09.05+09:00",
                DateTime::<FixedOffset>::from_naive_utc_and_offset(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2014, 11, 28).unwrap(),
                        NaiveTime::from_hms_milli_opt(12, 0, 9, 50).unwrap(),
                    ),
                    FixedOffset::east_opt(0).unwrap(),
                ),
            ),
            (
                "2014-11-28 21:00:09.05+09:00",
                DateTime::<FixedOffset>::from_naive_utc_and_offset(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(2014, 11, 28).unwrap(),
                        NaiveTime::from_hms_milli_opt(12, 0, 9, 50).unwrap(),
                    ),
                    FixedOffset::east_opt(0).unwrap(),
                ),
            ),
        ] {
            let input: InputValue = graphql_input_value!((raw));
            let parsed = DateTime::<FixedOffset>::from_input_value(&input);

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
            let parsed = DateTime::<FixedOffset>::from_input_value(&input);

            assert!(parsed.is_err(), "allows input: {input:?}");
        }
    }

    #[test]
    fn formats_correctly() {
        for (val, expected) in [
            (
                DateTime::<FixedOffset>::from_naive_utc_and_offset(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996, 12, 19).unwrap(),
                        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                    ),
                    FixedOffset::east_opt(0).unwrap(),
                ),
                graphql_input_value!("1996-12-19T00:00:00Z"),
            ),
            (
                DateTime::<FixedOffset>::from_naive_utc_and_offset(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1564, 1, 30).unwrap(),
                        NaiveTime::from_hms_milli_opt(5, 0, 0, 123).unwrap(),
                    ),
                    FixedOffset::east_opt(9 * 3600).unwrap(),
                ),
                graphql_input_value!("1564-01-30T05:00:00.123Z"),
            ),
        ] {
            let actual: InputValue = val.to_input_value();

            assert_eq!(actual, expected, "on value: {val}");
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

    use super::{Date, DateTime, FixedOffset, FromFixedOffset, LocalDateTime, LocalTime, TimeZone};

    #[tokio::test]
    async fn serializes() {
        #[derive(Clone, Copy)]
        struct CET;

        impl TimeZone for CET {
            type Offset = <chrono_tz::Tz as TimeZone>::Offset;

            fn from_offset(_: &Self::Offset) -> Self {
                CET
            }

            fn offset_from_local_date(
                &self,
                local: &chrono::NaiveDate,
            ) -> chrono::LocalResult<Self::Offset> {
                chrono_tz::CET.offset_from_local_date(local)
            }

            fn offset_from_local_datetime(
                &self,
                local: &chrono::NaiveDateTime,
            ) -> chrono::LocalResult<Self::Offset> {
                chrono_tz::CET.offset_from_local_datetime(local)
            }

            fn offset_from_utc_date(&self, utc: &chrono::NaiveDate) -> Self::Offset {
                chrono_tz::CET.offset_from_utc_date(utc)
            }

            fn offset_from_utc_datetime(&self, utc: &chrono::NaiveDateTime) -> Self::Offset {
                chrono_tz::CET.offset_from_utc_datetime(utc)
            }
        }

        impl FromFixedOffset for CET {
            fn from_fixed_offset(dt: DateTime<FixedOffset>) -> DateTime<Self> {
                dt.with_timezone(&CET)
            }
        }

        struct Root;

        #[graphql_object]
        impl Root {
            fn date() -> Date {
                Date::from_ymd_opt(2015, 3, 14).unwrap()
            }

            fn local_time() -> LocalTime {
                LocalTime::from_hms_opt(16, 7, 8).unwrap()
            }

            fn local_date_time() -> LocalDateTime {
                LocalDateTime::new(
                    Date::from_ymd_opt(2016, 7, 8).unwrap(),
                    LocalTime::from_hms_opt(9, 10, 11).unwrap(),
                )
            }

            fn date_time() -> DateTime<chrono::Utc> {
                DateTime::from_naive_utc_and_offset(
                    LocalDateTime::new(
                        Date::from_ymd_opt(1996, 12, 20).unwrap(),
                        LocalTime::from_hms_opt(0, 39, 57).unwrap(),
                    ),
                    chrono::Utc,
                )
            }

            fn pass_date_time(dt: DateTime<CET>) -> DateTime<CET> {
                dt
            }

            fn transform_date_time(dt: DateTime<CET>) -> DateTime<chrono::Utc> {
                dt.with_timezone(&chrono::Utc)
            }
        }

        const DOC: &str = r#"{
            date
            localTime
            localDateTime
            dateTime,
            passDateTime(dt: "2014-11-28T21:00:09+09:00")
            transformDateTime(dt: "2014-11-28T21:00:09+09:00")
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
                    "passDateTime": "2014-11-28T12:00:09Z",
                    "transformDateTime": "2014-11-28T12:00:09Z",
                }),
                vec![],
            )),
        );
    }
}
