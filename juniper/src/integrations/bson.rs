//! GraphQL support for [bson](https://github.com/mongodb/bson-rust) types.

use chrono::prelude::*;

use crate::{graphql_scalar, InputValue, ScalarValue, Value};

#[graphql_scalar(
    description = "ObjectId",
    with = object_id,
    parse_token = String,
)]
type ObjectId = bson::oid::ObjectId;

mod object_id {
    use super::*;

    pub(super) type Error = String;

    pub(super) fn to_output<S: ScalarValue>(v: &ObjectId) -> Value<S> {
        Value::scalar(v.to_hex())
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<ObjectId, Error> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                ObjectId::parse_str(s).map_err(|e| format!("Failed to parse `ObjectId`: {}", e))
            })
    }
}

#[graphql_scalar(
    description = "UtcDateTime",
    with = utc_date_time,
    parse_token = String,
)]
type UtcDateTime = bson::DateTime;

mod utc_date_time {
    use super::*;

    pub(super) type Error = String;

    pub(super) fn to_output<S: ScalarValue>(v: &UtcDateTime) -> Value<S> {
        Value::scalar((*v).to_chrono().to_rfc3339())
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<UtcDateTime, Error> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                s.parse::<DateTime<Utc>>()
                    .map_err(|e| format!("Failed to parse `UtcDateTime`: {}", e))
            })
            .map(UtcDateTime::from_chrono)
    }
}

#[cfg(test)]
mod test {
    use bson::{oid::ObjectId, DateTime as UtcDateTime};
    use chrono::{DateTime, Utc};

    use crate::{graphql_input_value, FromInputValue, InputValue};

    #[test]
    fn objectid_from_input_value() {
        let raw = "53e37d08776f724e42000000";
        let input: InputValue = graphql_input_value!((raw));

        let parsed: ObjectId = FromInputValue::from_input_value(&input).unwrap();
        let id = ObjectId::parse_str(raw).unwrap();

        assert_eq!(parsed, id);
    }

    #[test]
    fn utcdatetime_from_input_value() {
        let raw = "2020-03-23T17:38:32.446+00:00";
        let input: InputValue = graphql_input_value!((raw));

        let parsed: UtcDateTime = FromInputValue::from_input_value(&input).unwrap();
        let date_time = UtcDateTime::from_chrono(
            DateTime::parse_from_rfc3339(raw)
                .unwrap()
                .with_timezone(&Utc),
        );

        assert_eq!(parsed, date_time);
    }
}
