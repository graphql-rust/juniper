//! GraphQL support for [`jiff`] crate types.
//!
//! # Supported types
//!
//! | Rust type           | Format                | GraphQL scalar        |
//! |---------------------|-----------------------|-----------------------|
//! | [`civil::Date`]     | `yyyy-MM-dd`          | [`LocalDate`][s1]     |
//! | [`civil::Time`]     | `HH:mm[:ss[.SSS]]`    | [`LocalTime`][s2]     |
//! | [`civil::DateTime`] | `yyyy-MM-ddTHH:mm:ss` | [`LocalDateTime`][s3] |
//! | [`Zoned`]           | [RFC 3339] string     | [`DateTime`][s4]      |
//! | [`Timestamp`]       | [RFC 3339] string     | [`DateTime`][s4]      |
//!
//! [`civil::Date`]: jiff::civil::Date
//! [`civil::Time`]: jiff::civil::Time
//! [`civil::DateTime`]: jiff::civil::DateTime
//! [`Zoned`]: jiff::Zoned
//! [`Timestamp`]: jiff::Timestamp
//! [RFC 3339]: https://datatracker.ietf.org/doc/html/rfc3339#section-5.6
//! [s1]: https://graphql-scalars.dev/docs/scalars/local-date
//! [s2]: https://graphql-scalars.dev/docs/scalars/local-time
//! [s3]: https://graphql-scalars.dev/docs/scalars/local-date-time
//! [s4]: https://graphql-scalars.dev/docs/scalars/date-time

use crate::{graphql_scalar, InputValue, ScalarValue, Value};

/// A representation of a civil date in the Gregorian calendar.
///
/// A `Date` value corresponds to a triple of year, month and day. Every `Date`
/// value is guaranteed to be a valid Gregorian calendar date. For example,
/// both `2023-02-29` and `2023-11-31` are invalid and cannot be represented by
/// a `Date`.
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

/// A representation of civil "wall clock" time.
///
/// Conceptually, a `Time` value corresponds to the typical hours and minutes
/// that you might see on a clock. This type also contains the second and
/// fractional subsecond (to nanosecond precision) associated with a time.
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
                // `LocalTime` scalar only allows precision up to milliseconds.
                v.clone()
                    .round(jiff::Unit::Millisecond)
                    .unwrap_or_else(|e| panic!("failed to format `LocalTime`: {e}"))
                    .strftime(FORMAT)
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

/// A representation of a civil datetime in the Gregorian calendar.
///
/// A `DateTime` value corresponds to a pair of a [`Date`] and a [`Time`].
/// That is, a datetime contains a year, month, day, hour, minute, second and
/// the fractional number of nanoseconds.
///
/// A `DateTime` value is guaranteed to contain a valid date and time. For
/// example, neither `2023-02-29T00:00:00` nor `2015-06-30T23:59:60` are
/// valid `DateTime` values.
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
