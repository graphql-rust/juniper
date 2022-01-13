use std::fmt;

use chrono::{DateTime, TimeZone, Utc};
use juniper::{
    execute, graphql_value, EmptyMutation, EmptySubscription, FromInputValue, InputValue,
    ParseScalarResult, ParseScalarValue, RootNode, ScalarToken, ScalarValue, ToInputValue, Value,
    Variables,
};

use crate::custom_scalar::MyScalarValue;

#[derive(Debug, PartialEq, Eq, Hash, juniper::GraphQLScalar)]
#[graphql(
    scalar = MyScalarValue,
    specified_by_url = "https://tools.ietf.org/html/rfc4122",
)]
pub struct LargeId(i64);

#[derive(Debug, PartialEq, Eq, Hash, juniper::GraphQLScalar)]
#[graphql(scalar = MyScalarValue)]
pub struct SmallId {
    id: i32,
}

#[derive(juniper::GraphQLScalar)]
#[graphql(
    scalar = S: ScalarValue,
    specified_by_url = "https://tools.ietf.org/html/rfc3339",
    resolve = resolve_custom_date_time,
    from_input_value = custom_dt::from_input_value,
    from_input_value_err = String,
    from_str = from_str_custom_date_time,
)]
struct CustomDateTime<Tz>
where
    Tz: From<Utc> + TimeZone,
    Tz::Offset: fmt::Display,
{
    dt: DateTime<Tz>,
    _unused: (),
}

fn from_str_custom_date_time<S>(value: ScalarToken<'_>) -> ParseScalarResult<'_, S>
where
    S: ScalarValue,
{
    <String as ParseScalarValue<S>>::from_str(value)
}

fn resolve_custom_date_time<S, Tz>(dt: &CustomDateTime<Tz>) -> Value<S>
where
    S: ScalarValue,
    Tz: From<Utc> + TimeZone,
    Tz::Offset: fmt::Display,
{
    Value::scalar(dt.dt.to_rfc3339())
}

mod custom_dt {
    use super::{fmt, CustomDateTime, DateTime, InputValue, ScalarValue, TimeZone, Utc};

    pub(super) fn from_input_value<S, Tz>(v: &InputValue<S>) -> Result<CustomDateTime<Tz>, String>
    where
        S: ScalarValue,
        Tz: From<Utc> + TimeZone,
        Tz::Offset: fmt::Display,
    {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                DateTime::parse_from_rfc3339(s)
                    .map(|dt| CustomDateTime {
                        dt: dt.with_timezone(&Tz::from(Utc)),
                        _unused: (),
                    })
                    .map_err(|e| format!("Failed to parse CustomDateTime: {}", e))
            })
    }
}

#[derive(juniper::GraphQLObject)]
#[graphql(scalar = MyScalarValue)]
struct User {
    id: LargeId,
    another_id: SmallId,
}

struct Query;

#[juniper::graphql_object(scalar = MyScalarValue)]
impl Query {
    fn user() -> User {
        User {
            id: LargeId(0),
            another_id: SmallId { id: 0 },
        }
    }
}

struct Mutation;

#[juniper::graphql_object(scalar = MyScalarValue)]
impl Mutation {
    fn change_user(id: LargeId, another_id: SmallId) -> User {
        User { id, another_id }
    }
}

#[test]
fn test_scalar_value_large_id() {
    let num: i64 = 4294967297;

    let input_integer: InputValue<MyScalarValue> =
        serde_json::from_value(serde_json::json!(num)).unwrap();

    let output: LargeId =
        FromInputValue::<MyScalarValue>::from_input_value(&input_integer).unwrap();
    assert_eq!(output, LargeId(num));

    let id = LargeId(num);
    let output = ToInputValue::<MyScalarValue>::to_input_value(&id);
    assert_eq!(output, InputValue::scalar(num));
}

#[test]
fn test_scalar_value_small_id() {
    let num: i32 = i32::MAX;
    let id = SmallId { id: num };

    let input_integer: InputValue<MyScalarValue> =
        serde_json::from_value(serde_json::json!(num)).unwrap();

    let output: SmallId =
        FromInputValue::<MyScalarValue>::from_input_value(&input_integer).unwrap();
    assert_eq!(output, id);

    let output = ToInputValue::<MyScalarValue>::to_input_value(&id);
    assert_eq!(output, InputValue::scalar(num));
}

#[tokio::test]
async fn test_scalar_value_large_specified_url() {
    let schema = RootNode::<'_, _, _, _, MyScalarValue>::new_with_scalar_value(
        Query,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let doc = r#"{
        __type(name: "LargeId") {
            specifiedByUrl
        }
    }"#;

    assert_eq!(
        execute(doc, None, &schema, &Variables::<MyScalarValue>::new(), &()).await,
        Ok((
            graphql_value!({"__type": {"specifiedByUrl": "https://tools.ietf.org/html/rfc4122"}}),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_scalar_value_large_query() {
    let schema = RootNode::<'_, _, _, _, MyScalarValue>::new_with_scalar_value(
        Query,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let doc = r#"{
        user { id anotherId }
    }"#;

    let id = Value::<MyScalarValue>::scalar(0_i64);
    let another_id = Value::<MyScalarValue>::scalar(0_i32);
    assert_eq!(
        execute(doc, None, &schema, &Variables::<MyScalarValue>::new(), &()).await,
        Ok((
            graphql_value!({"user": {"id": id, "anotherId": another_id}}),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_scalar_value_large_mutation() {
    let schema = RootNode::<'_, _, _, _, MyScalarValue>::new_with_scalar_value(
        Query,
        Mutation,
        EmptySubscription::<()>::new(),
    );

    let doc = r#"mutation {
        changeUser(id: 1, anotherId: 2) { id anotherId }
    }"#;

    let id = Value::<MyScalarValue>::scalar(1_i64);
    let another_id = Value::<MyScalarValue>::scalar(2_i32);
    assert_eq!(
        execute(doc, None, &schema, &Variables::<MyScalarValue>::new(), &()).await,
        Ok((
            graphql_value!({"changeUser": {"id": id, "anotherId": another_id}}),
            vec![],
        )),
    );

    let doc = r#"mutation {
        changeUser(id: 4294967297, anotherId: -2147483648) { id anotherId }
    }"#;

    let id = Value::<MyScalarValue>::scalar(4294967297_i64);
    let another_id = Value::<MyScalarValue>::scalar(i32::MIN);
    assert_eq!(
        execute(doc, None, &schema, &Variables::<MyScalarValue>::new(), &()).await,
        Ok((
            graphql_value!({"changeUser": {"id": id, "anotherId": another_id}}),
            vec![],
        )),
    );
}
