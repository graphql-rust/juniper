use std::fmt;

use chrono::{DateTime, TimeZone, Utc};
use juniper::{
    execute, graphql_object, graphql_scalar, graphql_value, graphql_vars, DefaultScalarValue,
    InputValue, ParseScalarResult, ParseScalarValue, ScalarToken, ScalarValue, Value,
};

use crate::{
    custom_scalar::MyScalarValue,
    util::{schema, schema_with_scalar},
};

mod trivial {
    use super::*;

    #[graphql_scalar]
    struct Counter(i32);

    impl Counter {
        fn to_output<S: ScalarValue>(&self) -> Value<S> {
            Value::scalar(self.0)
        }

        fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Self, String> {
            v.as_int_value()
                .ok_or_else(|| format!("Expected `String`, found: {}", v))
                .map(Self)
        }

        fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
            <i32 as ParseScalarValue<S>>::from_str(value)
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

mod all_custom_resolvers {
    use super::*;

    #[graphql_scalar(
        to_output_with = to_output,
        from_input_with = from_input,
    )]
    #[graphql_scalar(
        parse_token_with = parse_token,
    )]
    struct Counter(i32);

    fn to_output<S: ScalarValue>(v: &Counter) -> Value<S> {
        Value::scalar(v.0)
    }

    fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Counter, String> {
        v.as_int_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .map(Counter)
    }

    fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
        <i32 as ParseScalarValue<S>>::from_str(value)
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

    #[graphql_scalar(name = "Counter")]
    struct CustomCounter(i32);

    impl CustomCounter {
        fn to_output<S: ScalarValue>(&self) -> Value<S> {
            Value::scalar(self.0)
        }

        fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Self, String> {
            v.as_int_value()
                .ok_or_else(|| format!("Expected `String`, found: {}", v))
                .map(Self)
        }

        fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
            <i32 as ParseScalarValue<S>>::from_str(value)
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

mod delegated_parse_token {
    use super::*;

    #[graphql_scalar(parse_token(i32))]
    struct Counter(i32);

    impl Counter {
        fn to_output<S: ScalarValue>(&self) -> Value<S> {
            Value::scalar(self.0)
        }

        fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Self, String> {
            v.as_int_value()
                .ok_or_else(|| format!("Expected `String`, found: {}", v))
                .map(Self)
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

mod multiple_delegated_parse_token {
    use super::*;

    #[graphql_scalar(parse_token(String, i32))]
    enum StringOrInt {
        String(String),
        Int(i32),
    }

    impl StringOrInt {
        fn to_output<S: ScalarValue>(&self) -> Value<S> {
            match self {
                StringOrInt::String(str) => Value::scalar(str.to_owned()),
                StringOrInt::Int(i) => Value::scalar(*i),
            }
        }

        fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Self, String> {
            v.as_string_value()
                .map(|s| Self::String(s.to_owned()))
                .or_else(|| v.as_int_value().map(|i| Self::Int(i)))
                .ok_or_else(|| format!("Expected `String` or `Int`, found: {}", v))
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn string_or_int(value: StringOrInt) -> StringOrInt {
            value
        }
    }

    #[tokio::test]
    async fn resolves_string() {
        const DOC: &str = r#"{ stringOrInt(value: "test") }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"stringOrInt": "test"}), vec![],)),
        );
    }

    #[tokio::test]
    async fn resolves_int() {
        const DOC: &str = r#"{ stringOrInt(value: 0) }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"stringOrInt": 0}), vec![],)),
        );
    }
}

mod where_attribute {
    use super::*;

    #[graphql_scalar(
        to_output_with = to_output,
        from_input_with = from_input,
        parse_token(String),
        where(Tz: From<Utc>, Tz::Offset: fmt::Display),
        specified_by_url = "https://tools.ietf.org/html/rfc3339",
    )]
    struct CustomDateTime<Tz: TimeZone>(DateTime<Tz>);

    fn to_output<S, Tz>(v: &CustomDateTime<Tz>) -> Value<S>
    where
        S: ScalarValue,
        Tz: From<Utc> + TimeZone,
        Tz::Offset: fmt::Display,
    {
        Value::scalar(v.0.to_rfc3339())
    }

    fn from_input<S, Tz>(v: &InputValue<S>) -> Result<CustomDateTime<Tz>, String>
    where
        S: ScalarValue,
        Tz: From<Utc> + TimeZone,
        Tz::Offset: fmt::Display,
    {
        v.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", v))
            .and_then(|s| {
                DateTime::parse_from_rfc3339(s)
                    .map(|dt| CustomDateTime(dt.with_timezone(&Tz::from(Utc))))
                    .map_err(|e| format!("Failed to parse CustomDateTime: {}", e))
            })
    }

    struct QueryRoot;

    #[graphql_object(scalar = MyScalarValue)]
    impl QueryRoot {
        fn date_time(value: CustomDateTime<Utc>) -> CustomDateTime<Utc> {
            value
        }
    }

    #[tokio::test]
    async fn resolves_custom_date_time() {
        const DOC: &str = r#"{ dateTime(value: "1996-12-19T16:39:57-08:00") }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot);

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

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"specifiedByUrl": "https://tools.ietf.org/html/rfc3339"}}),
                vec![],
            )),
        );
    }
}

mod with_self {
    use super::*;

    #[graphql_scalar(with = Self)]
    struct Counter(i32);

    impl Counter {
        fn to_output<S: ScalarValue>(&self) -> Value<S> {
            Value::scalar(self.0)
        }

        fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Self, String> {
            v.as_int_value()
                .ok_or_else(|| format!("Expected `String`, found: {}", v))
                .map(Self)
        }

        fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
            <i32 as ParseScalarValue<S>>::from_str(value)
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

mod with_module {
    use super::*;

    #[graphql_scalar(
        with = custom_date_time,
        parse_token(String),
        where(Tz: From<Utc>, Tz::Offset: fmt::Display),
        specified_by_url = "https://tools.ietf.org/html/rfc3339",
    )]
    struct CustomDateTime<Tz: TimeZone>(DateTime<Tz>);

    mod custom_date_time {
        use super::*;

        pub(super) fn to_output<S, Tz>(v: &CustomDateTime<Tz>) -> Value<S>
        where
            S: ScalarValue,
            Tz: From<Utc> + TimeZone,
            Tz::Offset: fmt::Display,
        {
            Value::scalar(v.0.to_rfc3339())
        }

        pub(super) fn from_input<S, Tz>(v: &InputValue<S>) -> Result<CustomDateTime<Tz>, String>
        where
            S: ScalarValue,
            Tz: From<Utc> + TimeZone,
            Tz::Offset: fmt::Display,
        {
            v.as_string_value()
                .ok_or_else(|| format!("Expected `String`, found: {}", v))
                .and_then(|s| {
                    DateTime::parse_from_rfc3339(s)
                        .map(|dt| CustomDateTime(dt.with_timezone(&Tz::from(Utc))))
                        .map_err(|e| format!("Failed to parse CustomDateTime: {}", e))
                })
        }
    }

    struct QueryRoot;

    #[graphql_object(scalar = MyScalarValue)]
    impl QueryRoot {
        fn date_time(value: CustomDateTime<Utc>) -> CustomDateTime<Utc> {
            value
        }
    }

    #[tokio::test]
    async fn resolves_custom_date_time() {
        const DOC: &str = r#"{ dateTime(value: "1996-12-19T16:39:57-08:00") }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot);

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

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot);

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

    /// Description
    #[graphql_scalar(with = counter)]
    struct Counter(i32);

    mod counter {
        use super::*;

        pub(super) fn to_output<S: ScalarValue>(v: &Counter) -> Value<S> {
            Value::scalar(v.0)
        }

        pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Counter, String> {
            v.as_int_value()
                .ok_or_else(|| format!("Expected `String`, found: {}", v))
                .map(Counter)
        }

        pub(super) fn parse_token<S: ScalarValue>(
            value: ScalarToken<'_>,
        ) -> ParseScalarResult<'_, S> {
            <i32 as ParseScalarValue<S>>::from_str(value)
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
                graphql_value!({"__type": {"description": "Description"}}),
                vec![]
            )),
        );
    }
}

mod description_from_attribute {
    use super::*;

    /// Doc comment
    #[graphql_scalar(description = "Description from attribute")]
    struct Counter(i32);

    impl Counter {
        fn to_output<S: ScalarValue>(&self) -> Value<S> {
            Value::scalar(self.0)
        }

        fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Self, String> {
            v.as_int_value()
                .ok_or_else(|| format!("Expected `String`, found: {}", v))
                .map(Self)
        }

        fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
            <i32 as ParseScalarValue<S>>::from_str(value)
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
                graphql_value!({"__type": {"description": "Description from attribute"}}),
                vec![]
            )),
        );
    }
}

mod custom_scalar {
    use super::*;

    /// Description
    #[graphql_scalar(
        scalar = MyScalarValue,
        with = counter,
    )]
    struct Counter(i32);

    mod counter {
        use super::*;

        pub(super) fn to_output<S: ScalarValue>(v: &Counter) -> Value<S> {
            Value::scalar(v.0)
        }

        pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Counter, String> {
            v.as_int_value()
                .ok_or_else(|| format!("Expected `String`, found: {}", v))
                .map(Counter)
        }

        pub(super) fn parse_token<S: ScalarValue>(
            value: ScalarToken<'_>,
        ) -> ParseScalarResult<'_, S> {
            <i32 as ParseScalarValue<S>>::from_str(value)
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
    async fn has_description() {
        const DOC: &str = r#"{
            __type(name: "Counter") {
                description
            }
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"description": "Description"}}),
                vec![]
            )),
        );
    }
}

mod generic_scalar {
    use super::*;

    /// Description
    #[graphql_scalar(
        scalar = S: ScalarValue,
        with = counter,
    )]
    struct Counter(i32);

    mod counter {
        use super::*;

        pub(super) fn to_output<S: ScalarValue>(v: &Counter) -> Value<S> {
            Value::scalar(v.0)
        }

        pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Counter, String> {
            v.as_int_value()
                .ok_or_else(|| format!("Expected `String`, found: {}", v))
                .map(Counter)
        }

        pub(super) fn parse_token<S: ScalarValue>(
            value: ScalarToken<'_>,
        ) -> ParseScalarResult<'_, S> {
            <i32 as ParseScalarValue<S>>::from_str(value)
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
    async fn has_description() {
        const DOC: &str = r#"{
            __type(name: "Counter") {
                description
            }
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"description": "Description"}}),
                vec![]
            )),
        );
    }
}

mod bounded_generic_scalar {
    use super::*;

    /// Description
    #[graphql_scalar(scalar = S: ScalarValue + Clone)]
    struct Counter(i32);

    impl Counter {
        fn to_output<S: ScalarValue>(&self) -> Value<S> {
            Value::scalar(self.0)
        }

        fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<Self, String> {
            v.as_int_value()
                .ok_or_else(|| format!("Expected `String`, found: {}", v))
                .map(Self)
        }

        fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
            <i32 as ParseScalarValue<S>>::from_str(value)
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
    async fn has_description() {
        const DOC: &str = r#"{
            __type(name: "Counter") {
                description
            }
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"description": "Description"}}),
                vec![]
            )),
        );
    }
}
