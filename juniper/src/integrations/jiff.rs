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
