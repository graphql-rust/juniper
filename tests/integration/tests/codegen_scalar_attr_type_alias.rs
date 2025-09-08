//! Tests for `#[graphql_scalar]` macro placed on a type alias.

pub mod common;

use std::fmt;

use chrono::{DateTime, TimeZone, Utc};
use juniper::{
    ParseScalarResult, ParseScalarValue, Scalar, ScalarToken, ScalarValue, execute, graphql_object,
    graphql_scalar, graphql_value, graphql_vars,
};

use self::common::{
    MyScalarValue,
    util::{schema, schema_with_scalar},
};

// Override `std::prelude` items to check whether macros expand hygienically.
use self::common::hygiene::*;

mod all_custom_resolvers {
    use super::*;

    struct CustomCounter(i32);

    #[graphql_scalar]
    #[graphql(
        to_output_with = to_output,
        from_input_with = CustomCounter,
    )]
    #[graphql(
        parse_token_with = parse_token,
    )]
    type Counter = CustomCounter;

    fn to_output(v: &Counter) -> i32 {
        v.0
    }

    fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<S> {
        <i32 as ParseScalarValue<S>>::from_str(value)
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

    #[graphql_scalar]
    #[graphql(
        name = "Counter",
        to_output_with = to_output,
        from_input_with = CustomCounter,
        parse_token_with = parse_token,
    )]
    type CounterScalar = CustomCounter;

    fn to_output(v: &CounterScalar) -> i32 {
        v.0
    }

    fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<S> {
        <i32 as ParseScalarValue<S>>::from_str(value)
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn counter(value: CounterScalar) -> CounterScalar {
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
        for name in ["CustomCounter", "CustomScalar"] {
            let doc = format!(
                r#"{{
                    __type(name: "{name}") {{
                        kind
                    }}
                }}"#,
            );

            let schema = schema(QueryRoot);

            assert_eq!(
                execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!(null), vec![])),
            );
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

    struct CustomCounter(i32);

    #[graphql_scalar]
    #[graphql(
        to_output_with = to_output,
        from_input_with = CustomCounter,
        parse_token(i32),
    )]
    type Counter = CustomCounter;

    fn to_output(v: &Counter) -> i32 {
        v.0
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

    enum StringOrIntScalar {
        String(prelude::String),
        Int(i32),
    }

    #[graphql_scalar]
    #[graphql(
        to_output_with = to_output,
        from_input_with = from_input,
        parse_token(prelude::String, i32),
    )]
    type StringOrInt = StringOrIntScalar;

    fn to_output<S: ScalarValue>(v: &StringOrInt) -> S {
        match v {
            StringOrInt::String(s) => S::from_displayable(s),
            StringOrInt::Int(i) => (*i).into(),
        }
    }

    fn from_input(v: &Scalar<impl ScalarValue>) -> prelude::Result<StringOrInt, prelude::Box<str>> {
        v.try_to_string()
            .map(StringOrInt::String)
            .or_else(|| v.try_to_int().map(StringOrInt::Int))
            .ok_or_else(|| format!("Expected `String` or `Int`, found: {v}").into())
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
            Ok((graphql_value!({"stringOrInt": "test"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_int() {
        const DOC: &str = r#"{ stringOrInt(value: 0) }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"stringOrInt": 0}), vec![])),
        );
    }
}

mod where_attribute {
    use super::*;

    struct CustomDateTimeScalar<Tz: TimeZone>(DateTime<Tz>);

    #[graphql_scalar]
    #[graphql(
        to_output_with = to_output,
        from_input_with = from_input,
        parse_token(prelude::String),
        where(Tz: From<Utc> + TimeZone, Tz::Offset: fmt::Display),
        specified_by_url = "https://tools.ietf.org/html/rfc3339",
    )]
    type CustomDateTime<Tz> = CustomDateTimeScalar<Tz>;

    fn to_output<Tz>(v: &CustomDateTime<Tz>) -> prelude::String
    where
        Tz: From<Utc> + TimeZone,
        Tz::Offset: fmt::Display,
    {
        v.0.to_rfc3339()
    }

    fn from_input<Tz>(s: &str) -> prelude::Result<CustomDateTime<Tz>, prelude::Box<str>>
    where
        Tz: From<Utc> + TimeZone,
        Tz::Offset: fmt::Display,
    {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| CustomDateTimeScalar(dt.with_timezone(&Tz::from(Utc))))
            .map_err(|e| format!("Failed to parse `CustomDateTime`: {e}").into())
    }

    struct QueryRoot;

    #[graphql_object]
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

    struct CustomCounter(i32);

    #[graphql_scalar]
    #[graphql(with = Self)]
    type Counter = CustomCounter;

    impl Counter {
        fn to_output(&self) -> i32 {
            self.0
        }

        fn from_input(i: i32) -> Self {
            Self(i)
        }

        fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<S> {
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

    struct CustomDateTimeScalar<Tz: TimeZone>(DateTime<Tz>);

    #[graphql_scalar]
    #[graphql(
        with = custom_date_time,
        parse_token(prelude::String),
        where(Tz: From<Utc> + TimeZone, Tz::Offset: fmt::Display),
        specified_by_url = "https://tools.ietf.org/html/rfc3339",
    )]
    type CustomDateTime<Tz> = CustomDateTimeScalar<Tz>;

    mod custom_date_time {
        use super::*;

        pub(super) fn to_output<Tz>(v: &CustomDateTime<Tz>) -> prelude::String
        where
            Tz: From<Utc> + TimeZone,
            Tz::Offset: fmt::Display,
        {
            v.0.to_rfc3339()
        }

        pub(super) fn from_input<Tz>(
            s: &str,
        ) -> prelude::Result<CustomDateTime<Tz>, prelude::Box<str>>
        where
            Tz: From<Utc> + TimeZone,
            Tz::Offset: fmt::Display,
        {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| CustomDateTimeScalar(dt.with_timezone(&Tz::from(Utc))))
                .map_err(|e| format!("Failed to parse `CustomDateTime`: {e}").into())
        }
    }

    struct QueryRoot;

    #[graphql_object]
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

    struct CustomCounter(i32);

    /// Description
    #[graphql_scalar]
    #[graphql(with = counter, parse_token(i32))]
    type Counter = CustomCounter;

    mod counter {
        use super::*;

        pub(super) fn to_output(v: &Counter) -> i32 {
            v.0
        }

        pub(super) fn from_input(i: i32) -> Counter {
            CustomCounter(i)
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
                vec![],
            )),
        );
    }
}

mod description_from_attribute {
    use super::*;

    struct CustomCounter(i32);

    /// Doc comment
    #[graphql_scalar]
    #[graphql(
        description = "Description from attribute",
        with = counter,
        parse_token(i32),
    )]
    type Counter = CustomCounter;

    mod counter {
        use super::*;

        pub(super) fn to_output(v: &Counter) -> i32 {
            v.0
        }

        pub(super) fn from_input(i: i32) -> Counter {
            CustomCounter(i)
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
                vec![],
            )),
        );
    }
}

mod custom_scalar {
    use super::*;

    struct CustomCounter(i32);

    /// Description
    #[graphql_scalar]
    #[graphql(
        scalar = MyScalarValue,
        with = counter,
        parse_token(i32),
    )]
    type Counter = CustomCounter;

    mod counter {
        use super::*;

        pub(super) fn to_output(v: &Counter) -> i32 {
            v.0
        }

        pub(super) fn from_input(i: i32) -> Counter {
            CustomCounter(i)
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
                vec![],
            )),
        );
    }
}

mod generic_scalar {
    use super::*;

    struct CustomCounter(i32);

    /// Description
    #[graphql_scalar]
    #[graphql(
        scalar = S: ScalarValue,
        with = counter,
        parse_token(i32),
    )]
    type Counter = CustomCounter;

    mod counter {
        use super::*;

        pub(super) fn to_output(v: &Counter) -> i32 {
            v.0
        }

        pub(super) fn from_input(i: i32) -> Counter {
            CustomCounter(i)
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
                vec![],
            )),
        );
    }
}

mod bounded_generic_scalar {
    use super::*;

    struct CustomCounter(i32);

    /// Description
    #[graphql_scalar]
    #[graphql(
        scalar = S: ScalarValue + prelude::Clone,
        with = counter,
        parse_token(i32),
    )]
    type Counter = CustomCounter;

    mod counter {
        use super::*;

        pub(super) fn to_output(v: &Counter) -> i32 {
            v.0
        }

        pub(super) fn from_input(i: i32) -> Counter {
            CustomCounter(i)
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
                vec![],
            )),
        );
    }
}
