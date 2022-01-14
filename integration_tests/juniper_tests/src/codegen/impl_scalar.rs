use std::fmt;

use chrono::{DateTime, TimeZone, Utc};
use juniper::{
    execute, graphql_object, graphql_scalar, graphql_value, graphql_vars, DefaultScalarValue,
    EmptyMutation, EmptySubscription, GraphQLScalar, GraphQLType, InputValue, ParseScalarResult,
    ParseScalarValue, RootNode, ScalarToken, ScalarValue, Value,
};

fn schema<'q, C, Q>(query_root: Q) -> RootNode<'q, Q, EmptyMutation<C>, EmptySubscription<C>>
where
    Q: GraphQLType<DefaultScalarValue, Context = C, TypeInfo = ()> + 'q,
{
    RootNode::new(
        query_root,
        EmptyMutation::<C>::new(),
        EmptySubscription::<C>::new(),
    )
}

fn schema_with_scalar<'q, S, C, Q>(
    query_root: Q,
) -> RootNode<'q, Q, EmptyMutation<C>, EmptySubscription<C>, S>
where
    Q: GraphQLType<S, Context = C, TypeInfo = ()> + 'q,
    S: ScalarValue + 'q,
{
    RootNode::new_with_scalar_value(
        query_root,
        EmptyMutation::<C>::new(),
        EmptySubscription::<C>::new(),
    )
}

mod trivial {
    use super::*;

    struct Counter(i32);

    #[graphql_scalar]
    impl GraphQLScalar for Counter {
        type Error = String;

        fn to_output(&self) -> Value {
            Value::scalar(self.0)
        }

        fn from_input(v: &InputValue) -> Result<Self, Self::Error> {
            v.as_int_value()
                .map(Self)
                .ok_or_else(|| format!("Expected `Int`, found: {}", v))
        }

        fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<'_> {
            <i32 as ParseScalarValue>::from_str(value)
        }
    }

    struct QueryRoot;

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn counter(value: Counter) -> Counter {
            value
        }
    }

    #[tokio::test]
    async fn is_graphql_scalar() {
        const DOC: &str = r#"{
            __type(name: "Counter") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "SCALAR"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_counter() {
        const DOC: &str = r#"{ counter(value: 0) }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"counter": 0}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Counter") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"description": null}}), vec![])),
        );
    }
}

mod explicit_name {
    use super::*;

    struct CustomCounter(i32);

    #[graphql_scalar(name = "Counter")]
    impl GraphQLScalar for CustomCounter {
        type Error = String;

        fn to_output(&self) -> Value {
            Value::scalar(self.0)
        }

        fn from_input(v: &InputValue) -> Result<Self, Self::Error> {
            v.as_int_value()
                .map(Self)
                .ok_or_else(|| format!("Expected `Int`, found: {}", v))
        }

        fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<'_> {
            <i32 as ParseScalarValue>::from_str(value)
        }
    }

    struct QueryRoot;

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn counter(value: CustomCounter) -> CustomCounter {
            value
        }
    }

    #[tokio::test]
    async fn is_graphql_scalar() {
        const DOC: &str = r#"{
            __type(name: "Counter") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "SCALAR"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn no_custom_counter() {
        const DOC: &str = r#"{
            __type(name: "CustomCounter") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!(null), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_counter() {
        const DOC: &str = r#"{ counter(value: 0) }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"counter": 0}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Counter") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"description": null}}), vec![])),
        );
    }
}

mod generic {
    use super::*;

    struct CustomDateTime<Tz: TimeZone>(DateTime<Tz>);

    #[graphql_scalar(specified_by_url = "https://tools.ietf.org/html/rfc3339")]
    impl<S, Tz> GraphQLScalar<S> for CustomDateTime<Tz>
    where
        S: ScalarValue,
        Tz: From<Utc> + TimeZone,
        Tz::Offset: fmt::Display,
    {
        type Error = String;

        fn to_output(&self) -> Value<S> {
            Value::scalar(self.0.to_rfc3339())
        }

        fn from_input(v: &InputValue<S>) -> Result<Self, Self::Error> {
            v.as_string_value()
                .ok_or_else(|| format!("Expected `String`, found: {}", v))
                .and_then(|s| {
                    DateTime::parse_from_rfc3339(s)
                        .map(|dt| Self(dt.with_timezone(&Tz::from(Utc))))
                        .map_err(|e| format!("Failed to parse CustomDateTime: {}", e))
                })
        }

        fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
            <String as ParseScalarValue<S>>::from_str(value)
        }
    }

    struct QueryRoot;

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn date_time(value: CustomDateTime<Utc>) -> CustomDateTime<Utc> {
            value
        }
    }

    #[tokio::test]
    async fn resolves_custom_date_time() {
        const DOC: &str = r#"{ dateTime(value: "1996-12-19T16:39:57-08:00") }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"dateTime": "1996-12-20T00:39:57+00:00"}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_specified_by_url() {
        const DOC: &str = r#"{
            __type(name: "CustomDateTime") {
                specifiedByUrl
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"specifiedByUrl": "https://tools.ietf.org/html/rfc3339"}}),
                vec![],
            )),
        );
    }
}

mod description_from_doc_comment {
    use super::*;

    struct Counter(i32);

    /// Doc comment.
    #[graphql_scalar]
    impl GraphQLScalar for Counter {
        type Error = String;

        fn to_output(&self) -> Value {
            Value::scalar(self.0)
        }

        fn from_input(v: &InputValue) -> Result<Self, Self::Error> {
            v.as_int_value()
                .map(Self)
                .ok_or_else(|| format!("Expected `Int`, found: {}", v))
        }

        fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<'_> {
            <i32 as ParseScalarValue>::from_str(value)
        }
    }

    struct QueryRoot;

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn counter(value: Counter) -> Counter {
            value
        }
    }

    #[tokio::test]
    async fn resolves_counter() {
        const DOC: &str = r#"{ counter(value: 0) }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"counter": 0}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_description() {
        const DOC: &str = r#"{
            __type(name: "Counter") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"description": "Doc comment."}}),
                vec![],
            )),
        );
    }
}

mod description_from_attribute {
    use super::*;

    struct Counter(i32);

    /// Doc comment.
    #[graphql_scalar(desc = "Doc comment from attribute.")]
    #[graphql_scalar(specified_by_url = "https://tools.ietf.org/html/rfc4122")]
    impl GraphQLScalar for Counter {
        type Error = String;

        fn to_output(&self) -> Value {
            Value::scalar(self.0)
        }

        fn from_input(v: &InputValue) -> Result<Self, Self::Error> {
            v.as_int_value()
                .map(Self)
                .ok_or_else(|| format!("Expected `Int`, found: {}", v))
        }

        fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<'_> {
            <i32 as ParseScalarValue>::from_str(value)
        }
    }

    struct QueryRoot;

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn counter(value: Counter) -> Counter {
            value
        }
    }

    #[tokio::test]
    async fn resolves_counter() {
        const DOC: &str = r#"{ counter(value: 0) }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"counter": 0}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_description_and_url() {
        const DOC: &str = r#"{
            __type(name: "Counter") {
                description
                specifiedByUrl
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({
                    "__type": {
                        "description": "Doc comment from attribute.",
                        "specifiedByUrl": "https://tools.ietf.org/html/rfc4122",
                    }
                }),
                vec![],
            )),
        );
    }
}

mod custom_scalar {
    use crate::custom_scalar::MyScalarValue;

    use super::*;

    struct Counter(i32);

    #[graphql_scalar]
    impl GraphQLScalar<MyScalarValue> for Counter {
        type Error = String;

        fn to_output(&self) -> Value<MyScalarValue> {
            Value::scalar(self.0)
        }

        fn from_input(v: &InputValue<MyScalarValue>) -> Result<Self, Self::Error> {
            v.as_int_value()
                .map(Self)
                .ok_or_else(|| format!("Expected `Int`, found: {}", v))
        }

        fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<'_, MyScalarValue> {
            <i32 as ParseScalarValue<MyScalarValue>>::from_str(value)
        }
    }

    struct QueryRoot;

    #[graphql_object(scalar = MyScalarValue)]
    impl QueryRoot {
        fn counter(value: Counter) -> Counter {
            value
        }
    }

    #[tokio::test]
    async fn is_graphql_scalar() {
        const DOC: &str = r#"{
            __type(name: "Counter") {
                kind
            }
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "SCALAR"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_counter() {
        const DOC: &str = r#"{ counter(value: 0) }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"counter": 0}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Counter") {
                description
            }
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"description": null}}), vec![])),
        );
    }
}

mod generic_scalar {
    use super::*;

    struct Counter(i32);

    #[graphql_scalar]
    impl<S> GraphQLScalar<S> for Counter
    where
        S: ScalarValue,
    {
        type Error = String;

        fn to_output(&self) -> Value<S> {
            Value::scalar(self.0)
        }

        fn from_input(v: &InputValue<S>) -> Result<Self, Self::Error> {
            v.as_int_value()
                .map(Self)
                .ok_or_else(|| format!("Expected `Int`, found: {}", v))
        }

        fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
            <i32 as ParseScalarValue<S>>::from_str(value)
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn counter(value: Counter) -> Counter {
            value
        }
    }

    #[tokio::test]
    async fn is_graphql_scalar() {
        const DOC: &str = r#"{
            __type(name: "Counter") {
                kind
            }
        }"#;

        let schema = schema_with_scalar::<DefaultScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "SCALAR"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_counter() {
        const DOC: &str = r#"{ counter(value: 0) }"#;

        let schema = schema_with_scalar::<DefaultScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"counter": 0}), vec![])),
        );
    }
}

mod bounded_generic_scalar {
    use super::*;

    struct Counter(i32);

    #[graphql_scalar]
    impl<S> GraphQLScalar<S> for Counter
    where
        S: ScalarValue + Clone,
    {
        type Error = String;

        fn to_output(&self) -> Value<S> {
            Value::scalar(self.0)
        }

        fn from_input(v: &InputValue<S>) -> Result<Self, Self::Error> {
            v.as_int_value()
                .map(Self)
                .ok_or_else(|| format!("Expected `Int`, found: {}", v))
        }

        fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
            <i32 as ParseScalarValue<S>>::from_str(value)
        }
    }

    struct QueryRoot;

    #[graphql_object(scalar = S: ScalarValue + Clone)]
    impl QueryRoot {
        fn counter(value: Counter) -> Counter {
            value
        }
    }

    #[tokio::test]
    async fn is_graphql_scalar() {
        const DOC: &str = r#"{
            __type(name: "Counter") {
                kind
            }
        }"#;

        let schema = schema_with_scalar::<DefaultScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "SCALAR"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_counter() {
        const DOC: &str = r#"{ counter(value: 0) }"#;

        let schema = schema_with_scalar::<DefaultScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"counter": 0}), vec![])),
        );
    }
}
