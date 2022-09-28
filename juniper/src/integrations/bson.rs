//! GraphQL support for [bson](https://github.com/mongodb/bson-rust) types.

use crate::{graphql_scalar, InputValue, ScalarValue, Value};

#[graphql_scalar(with = object_id, parse_token(String))]
type ObjectId = bson::oid::ObjectId;

mod object_id {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &ObjectId) -> Value<S> {
        Value::scalar(v.to_hex())
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<ObjectId, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| {
                ObjectId::parse_str(s).map_err(|e| format!("Failed to parse `ObjectId`: {e}"))
            })
    }
}

#[graphql_scalar(with = utc_date_time, parse_token(String))]
type UtcDateTime = bson::DateTime;

mod utc_date_time {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &UtcDateTime) -> Value<S> {
        Value::scalar(
            (*v).try_to_rfc3339_string()
                .unwrap_or_else(|e| panic!("failed to format `UtcDateTime` as RFC3339: {e}")),
        )
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<UtcDateTime, String> {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {v}"))
            .and_then(|s| {
                UtcDateTime::parse_rfc3339_str(s)
                    .map_err(|e| format!("Failed to parse `UtcDateTime`: {e}"))
            })
    }
}

#[cfg(test)]
mod test {
    use bson::{oid::ObjectId, DateTime as UtcDateTime};

    use crate::{graphql_input_value, FromInputValue, InputValue};

    #[test]
    fn objectid_from_input() {
        let raw = "53e37d08776f724e42000000";
        let input: InputValue = graphql_input_value!((raw));

        let parsed: ObjectId = FromInputValue::from_input_value(&input).unwrap();
        let id = ObjectId::parse_str(raw).unwrap();

        assert_eq!(parsed, id);
    }

    #[test]
    fn utcdatetime_from_input() {
        use chrono::{DateTime, Utc};

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
