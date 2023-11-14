//! Tests for `#[graphql_object]` macro.

// Assert that `#[graphql_object]` macro placed on a `impl` stops Clippy from enforcing `# Errors`
// and `# Panics` sections in GraphQL descriptions.
#![deny(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod common;

use juniper::{
    execute, graphql_object, graphql_value, graphql_vars, DefaultScalarValue, Executor, FieldError,
    FieldResult, GraphQLInputObject, GraphQLObject, IntoFieldError, ScalarValue,
};

use self::common::util::{schema, schema_with_scalar};

// Override `std::prelude` items to check whether macros expand hygienically.
#[allow(unused_imports)]
use self::common::hygiene::*;

mod trivial {
    use super::*;

    struct Human;

    #[graphql_object]
    impl Human {
        fn id() -> &'static str {
            "human-32"
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            human {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"human": {"id": "human-32"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_graphql_object() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "OBJECT"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_type_name() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                name
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Human"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Human") {
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

mod trivial_async {
    use futures::future;

    use super::*;

    struct Human;

    #[graphql_object]
    impl Human {
        async fn id() -> &'static str {
            future::ready("human-32").await
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            human {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"human": {"id": "human-32"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_graphql_object() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "OBJECT"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_type_name() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                name
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Human"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Human") {
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

mod raw_method {
    use super::*;

    struct Human;

    #[graphql_object]
    impl Human {
        fn r#my_id() -> &'static str {
            "human-32"
        }

        async fn r#async() -> &'static str {
            "async-32"
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            human {
                myId
                async
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"human": {"myId": "human-32", "async": "async-32"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_correct_name() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                name
                kind
                fields {
                    name
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "name": "Human",
                    "kind": "OBJECT",
                    "fields": [{"name": "myId"}, {"name": "async"}],
                }}),
                vec![],
            )),
        );
    }
}

mod ignored_method {
    use super::*;

    struct Human;

    #[graphql_object]
    impl Human {
        fn id() -> &'static str {
            "human-32"
        }

        #[allow(dead_code)]
        #[graphql(ignore)]
        fn planet() -> &'static str {
            "earth"
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            human {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"human": {"id": "human-32"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_not_field() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                fields {
                    name
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [{"name": "id"}]}}),
                vec![],
            )),
        );
    }
}

mod fallible_method {
    use super::*;

    struct CustomError;

    impl<S: ScalarValue> IntoFieldError<S> for CustomError {
        fn into_field_error(self) -> FieldError<S> {
            FieldError::new("Whatever", graphql_value!({"code": "some"}))
        }
    }

    struct Human {
        id: prelude::String,
    }

    #[graphql_object]
    impl Human {
        fn id(&self) -> prelude::Result<&str, CustomError> {
            Ok(&self.id)
        }

        async fn home_planet<__S>() -> FieldResult<&'static str, __S> {
            Ok("earth")
        }
    }

    #[derive(Clone, Copy)]
    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human {
                id: "human-32".into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            human {
                id
                homePlanet
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"human": {"id": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_correct_graphql_type() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                name
                kind
                fields {
                    name
                    type {
                        kind
                        ofType {
                            name
                        }
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "name": "Human",
                    "kind": "OBJECT",
                    "fields": [{
                        "name": "id",
                        "type": {"kind": "NON_NULL", "ofType": {"name": "String"}},
                    }, {
                        "name": "homePlanet",
                        "type": {"kind": "NON_NULL", "ofType": {"name": "String"}},
                    }]
                }}),
                vec![],
            )),
        );
    }
}

mod generic {
    use super::*;

    struct Human<A = (), B: ?Sized = ()> {
        id: A,
        _home_planet: B,
    }

    #[graphql_object]
    impl<B: ?Sized> Human<i32, B> {
        fn id(&self) -> i32 {
            self.id
        }
    }

    #[graphql_object(name = "HumanString")]
    impl<B: ?Sized> Human<prelude::String, B> {
        fn id(&self) -> &str {
            self.id.as_str()
        }
    }

    #[derive(Clone, Copy)]
    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human(&self) -> Human<i32> {
            Human {
                id: 32,
                _home_planet: (),
            }
        }

        fn human_string(&self) -> Human<prelude::String> {
            Human {
                id: "human-32".into(),
                _home_planet: (),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            human {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"human": {"id": 32}}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_human_string() {
        const DOC: &str = r#"{
            humanString {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"humanString": {"id": "human-32"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_type_name_without_type_params() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                name
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Human"}}), vec![])),
        );
    }
}

mod generic_async {
    use super::*;

    struct Human<A = (), B: ?Sized = ()> {
        id: A,
        _home_planet: B,
    }

    #[graphql_object]
    impl<B: ?Sized> Human<i32, B> {
        async fn id(&self) -> i32 {
            self.id
        }
    }

    #[graphql_object(name = "HumanString")]
    impl<B: ?Sized> Human<prelude::String, B> {
        async fn id(&self) -> &str {
            self.id.as_str()
        }
    }

    #[derive(Clone, Copy)]
    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human(&self) -> Human<i32> {
            Human {
                id: 32,
                _home_planet: (),
            }
        }

        fn human_string(&self) -> Human<prelude::String> {
            Human {
                id: "human-32".into(),
                _home_planet: (),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            human {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"human": {"id": 32}}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_human_string() {
        const DOC: &str = r#"{
            humanString {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"humanString": {"id": "human-32"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_type_name_without_type_params() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                name
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Human"}}), vec![])),
        );
    }
}

mod generic_lifetime_async {
    use super::*;

    struct Human<'p, A = ()> {
        id: A,
        home_planet: &'p str,
    }

    #[graphql_object]
    impl<'p> Human<'p, i32> {
        async fn id(&self) -> i32 {
            self.id
        }

        async fn planet(&self) -> &str {
            self.home_planet
        }
    }

    #[graphql_object(name = "HumanString")]
    impl<'id, 'p> Human<'p, &'id str> {
        async fn id(&self) -> &str {
            self.id
        }

        async fn planet(&self) -> &str {
            self.home_planet
        }
    }

    #[derive(Clone)]
    struct QueryRoot(prelude::String);

    #[graphql_object]
    impl QueryRoot {
        fn human(&self) -> Human<'static, i32> {
            Human {
                id: 32,
                home_planet: "earth",
            }
        }

        fn human_string(&self) -> Human<'_, &str> {
            Human {
                id: self.0.as_str(),
                home_planet: self.0.as_str(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            human {
                id
                planet
            }
        }"#;

        let schema = schema(QueryRoot("mars".into()));

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"human": {"id": 32, "planet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_human_string() {
        const DOC: &str = r#"{
            humanString {
                id
                planet
            }
        }"#;

        let schema = schema(QueryRoot("mars".into()));

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"humanString": {"id": "mars", "planet": "mars"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_type_name_without_type_params() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                name
            }
        }"#;

        let schema = schema(QueryRoot("mars".into()));

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Human"}}), vec![])),
        );
    }
}

mod nested_generic_lifetime_async {
    use super::*;

    struct Droid<'p, A = ()> {
        id: A,
        primary_function: &'p str,
    }

    #[graphql_object]
    impl<'p> Droid<'p, i32> {
        async fn id(&self) -> i32 {
            self.id
        }

        async fn primary_function(&self) -> &str {
            self.primary_function
        }
    }

    #[graphql_object(name = "DroidString")]
    impl<'id, 'p> Droid<'p, &'id str> {
        async fn id(&self) -> &str {
            self.id
        }

        async fn primary_function(&self) -> &str {
            self.primary_function
        }
    }

    struct Human<'p, A = ()> {
        id: A,
        home_planet: &'p str,
    }

    #[graphql_object]
    impl<'p> Human<'p, i32> {
        async fn id(&self) -> i32 {
            self.id
        }

        async fn planet(&self) -> &str {
            self.home_planet
        }

        async fn droid(&self) -> Droid<'_, i32> {
            Droid {
                id: self.id,
                primary_function: "run",
            }
        }
    }

    #[graphql_object(name = "HumanString")]
    impl<'id, 'p> Human<'p, &'id str> {
        async fn id(&self) -> &str {
            self.id
        }

        async fn planet(&self) -> &str {
            self.home_planet
        }

        async fn droid(&self) -> Droid<'_, &str> {
            Droid {
                id: "none",
                primary_function: self.home_planet,
            }
        }
    }

    #[derive(Clone)]
    struct QueryRoot(prelude::String);

    #[graphql_object]
    impl QueryRoot {
        fn human(&self) -> Human<'static, i32> {
            Human {
                id: 32,
                home_planet: "earth",
            }
        }

        fn human_string(&self) -> Human<'_, &str> {
            Human {
                id: self.0.as_str(),
                home_planet: self.0.as_str(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            human {
                id
                planet
                droid {
                    id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot("mars".into()));

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"human": {
                    "id": 32,
                    "planet": "earth",
                    "droid": {
                        "id": 32,
                        "primaryFunction": "run",
                    },
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_human_string() {
        const DOC: &str = r#"{
            humanString {
                id
                planet
                droid {
                    id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot("mars".into()));

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"humanString": {
                    "id": "mars",
                    "planet": "mars",
                    "droid": {
                        "id": "none",
                        "primaryFunction": "mars",
                    },
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_type_name_without_type_params() {
        for object in ["Human", "Droid"] {
            let doc = format!(
                r#"{{
                    __type(name: "{object}") {{
                        name
                    }}
                }}"#,
            );

            let schema = schema(QueryRoot("mars".into()));

            assert_eq!(
                execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"__type": {"name": object}}), vec![])),
            );
        }
    }
}

mod argument {
    use super::*;

    struct Human;

    #[graphql_object]
    impl Human {
        fn id(arg: prelude::String) -> prelude::String {
            arg
        }

        async fn home_planet(
            &self,
            r#raw_arg: prelude::String,
            r#async: prelude::Option<i32>,
        ) -> prelude::String {
            format!("{raw_arg},{async:?}")
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            human {
                id(arg: "human-32")
                homePlanet(rawArg: "earth")
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"human": {"id": "human-32", "homePlanet": "earth,None"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_correct_name() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                fields {
                    name
                    args {
                        name
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "id", "args": [{"name": "arg"}]},
                    {"name": "homePlanet", "args": [{"name": "rawArg"}, {"name": "async"}]},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                fields {
                    args {
                        description
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"args": [{"description": null}]},
                    {"args": [{"description": null}, {"description": null}]},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_no_defaults() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                fields {
                    args {
                        defaultValue
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"args": [{"defaultValue": null}]},
                    {"args": [{"defaultValue": null}, {"defaultValue": null}]},
                ]}}),
                vec![],
            )),
        );
    }
}

mod default_argument {
    use super::*;

    #[derive(GraphQLInputObject, Debug)]
    struct Point {
        x: i32,
    }

    struct Human;

    #[graphql_object]
    impl Human {
        fn id(
            #[graphql(default)] arg1: i32,
            #[graphql(default = "second".to_string())] arg2: prelude::Option<prelude::String>,
            #[graphql(default = true)] r#arg3: bool,
        ) -> prelude::String {
            format!("{arg1}|{arg2:?}&{arg3}")
        }

        fn info(#[graphql(default = Point { x: 1 })] coord: Point) -> i32 {
            coord.x
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        let schema = schema(QueryRoot);

        for (input, expected, vars) in [
            (
                "{ human { id } }",
                r#"0|Some("second")&true"#,
                graphql_vars! {},
            ),
            (
                "{ human { id(arg1: 1) } }",
                r#"1|Some("second")&true"#,
                graphql_vars! {},
            ),
            (
                r#"{ human { id(arg2: "other") } }"#,
                r#"0|Some("other")&true"#,
                graphql_vars! {},
            ),
            (
                "{ human { id(arg2: null) } }",
                r#"0|None&true"#,
                graphql_vars! {},
            ),
            (
                "query q($arg2: String) { human { id(arg2: $arg2) } }",
                r#"0|Some("second")&true"#,
                graphql_vars! {},
            ),
            (
                "query q($arg2: String) { human{ id(arg2: $arg2) } }",
                r#"0|None&true"#,
                graphql_vars! { "arg2": null },
            ),
            (
                "query q($arg2: String) { human{ id(arg2: $arg2) } }",
                r#"0|Some("other")&true"#,
                graphql_vars! { "arg2": "other" },
            ),
            (
                r#"query q($arg2: String = "other") { human { id(arg2: $arg2) } }"#,
                r#"0|Some("other")&true"#,
                graphql_vars! {},
            ),
            (
                r#"query q($arg2: String = "other") { human { id(arg2: $arg2) } }"#,
                r#"0|None&true"#,
                graphql_vars! { "arg2": null },
            ),
            (
                r#"query q($arg2: String = "other") { human { id(arg2: $arg2) } }"#,
                r#"0|Some("hello")&true"#,
                graphql_vars! { "arg2": "hello" },
            ),
            (
                r#"{ human { id(arg1: 2, arg2: "") } }"#,
                r#"2|Some("")&true"#,
                graphql_vars! {},
            ),
            (
                r#"{ human { id(arg1: 1, arg2: "", arg3: false) } }"#,
                r#"1|Some("")&false"#,
                graphql_vars! {},
            ),
        ] {
            assert_eq!(
                execute(input, None, &schema, &vars, &(),).await,
                Ok((graphql_value!({"human": {"id": expected}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn resolves_info_field() {
        let schema = schema(QueryRoot);

        for (input, expected) in [
            ("{ human { info } }", 1),
            ("{ human { info(coord: { x: 2 }) } }", 2),
        ] {
            assert_eq!(
                execute(input, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"human": {"info": expected}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn has_defaults() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                fields {
                    args {
                        name
                        defaultValue
                        type {
                            name
                            ofType {
                                name
                            }
                        }
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [{
                    "args": [{
                        "name": "arg1",
                        "defaultValue": "0",
                        "type": {"name": null, "ofType": {"name": "Int"}},
                    }, {
                        "name": "arg2",
                        "defaultValue": r#""second""#,
                        "type": {"name": "String", "ofType": null},
                    }, {
                        "name": "arg3",
                        "defaultValue": "true",
                        "type": {"name": null, "ofType": {"name": "Boolean"}},
                    }],
                }, {
                    "args": [{
                        "name": "coord",
                        "defaultValue": "{x: 1}",
                        "type": {"name": null, "ofType": {"name": "Point"}},
                    }],
                }]}}),
                vec![],
            )),
        );
    }
}

mod description_from_doc_comment {
    use super::*;

    struct Human;

    /// Rust docs.
    #[graphql_object]
    impl Human {
        /// Rust `id` docs.
        fn id() -> &'static str {
            "human-32"
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn uses_doc_comment_as_description() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                description
                fields {
                    description
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "description": "Rust docs.",
                    "fields": [{"description": "Rust `id` docs."}],
                }}),
                vec![],
            )),
        );
    }
}

mod deprecation_from_attr {
    use super::*;

    struct Human;

    #[graphql_object]
    impl Human {
        fn id() -> &'static str {
            "human-32"
        }

        #[deprecated]
        fn a(&self) -> &str {
            "a"
        }

        #[deprecated(note = "Use `id`.")]
        fn b(&self) -> &str {
            "b"
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            human {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"human": {"id": "human-32"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_deprecated_fields() {
        const DOC: &str = r#"{
            human {
                a
                b
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"human": {"a": "a", "b": "b"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn deprecates_fields() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                fields(includeDeprecated: true) {
                    name
                    isDeprecated
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "id", "isDeprecated": false},
                    {"name": "a", "isDeprecated": true},
                    {"name": "b", "isDeprecated": true},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn provides_deprecation_reason() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                fields(includeDeprecated: true) {
                    name
                    deprecationReason
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "id", "deprecationReason": null},
                    {"name": "a", "deprecationReason": null},
                    {"name": "b", "deprecationReason": "Use `id`."},
                ]}}),
                vec![],
            )),
        );
    }
}

mod explicit_name_description_and_deprecation {
    use super::*;

    struct Human;

    /// Rust docs.
    #[graphql_object(name = "MyHuman", desc = "My human.")]
    impl Human {
        /// Rust `id` docs.
        #[graphql(name = "myId", desc = "My human ID.", deprecated = "Not used.")]
        #[deprecated(note = "Should be omitted.")]
        fn id(
            #[graphql(name = "myName", desc = "My argument.", default)] _n: prelude::String,
        ) -> &'static str {
            "human-32"
        }

        #[graphql(deprecated)]
        #[deprecated(note = "Should be omitted.")]
        fn a(&self) -> &str {
            "a"
        }

        fn b(&self) -> &str {
            "b"
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            human {
                myId
                a
                b
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"human": {"myId": "human-32", "a": "a", "b": "b"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_custom_name() {
        const DOC: &str = r#"{
            __type(name: "MyHuman") {
                name
                fields(includeDeprecated: true) {
                    name
                    args {
                        name
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "name": "MyHuman",
                    "fields": [
                        {"name": "myId", "args": [{"name": "myName"}]},
                        {"name": "a", "args": []},
                        {"name": "b", "args": []},
                    ],
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_custom_description() {
        const DOC: &str = r#"{
            __type(name: "MyHuman") {
                description
                fields(includeDeprecated: true) {
                    name
                    description
                    args {
                        description
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "description": "My human.",
                    "fields": [{
                        "name": "myId",
                        "description": "My human ID.",
                        "args": [{"description": "My argument."}],
                    }, {
                        "name": "a",
                        "description": null,
                        "args": [],
                    }, {
                        "name": "b",
                        "description": null,
                        "args": [],
                    }],
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_custom_deprecation() {
        const DOC: &str = r#"{
            __type(name: "MyHuman") {
                fields(includeDeprecated: true) {
                    name
                    isDeprecated
                    deprecationReason
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "fields": [{
                        "name": "myId",
                        "isDeprecated": true,
                        "deprecationReason": "Not used.",
                    }, {
                        "name": "a",
                        "isDeprecated": true,
                        "deprecationReason": null,
                    }, {
                        "name": "b",
                        "isDeprecated": false,
                        "deprecationReason": null,
                    }],
                }}),
                vec![],
            )),
        );
    }
}

mod renamed_all_fields_and_args {
    use super::*;

    struct Human;

    #[graphql_object(rename_all = "none")]
    impl Human {
        fn id() -> &'static str {
            "human-32"
        }

        async fn home_planet(&self, planet_name: prelude::String) -> prelude::String {
            planet_name
        }

        async fn r#async_info(r#my_num: i32) -> i32 {
            r#my_num
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            human {
                id
                home_planet(planet_name: "earth")
                async_info(my_num: 3)
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"human": {
                    "id": "human-32",
                    "home_planet": "earth",
                    "async_info": 3,
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_correct_fields_and_args_names() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                fields {
                    name
                    args {
                        name
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "id", "args": []},
                    {"name": "home_planet", "args": [{"name": "planet_name"}]},
                    {"name": "async_info", "args": [{"name": "my_num"}]},
                ]}}),
                vec![],
            )),
        );
    }
}

mod explicit_scalar {
    use super::*;

    struct Human;

    #[graphql_object(scalar = DefaultScalarValue)]
    impl Human {
        fn id(&self) -> &str {
            "human-32"
        }

        async fn home_planet() -> &'static str {
            "earth"
        }
    }

    struct QueryRoot;

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            human {
                id
                homePlanet
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"human": {"id": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }
}

mod custom_scalar {
    use crate::common::MyScalarValue;

    use super::*;

    struct Human;

    #[graphql_object(scalar = MyScalarValue)]
    impl Human {
        fn id() -> &'static str {
            "human-32"
        }

        async fn home_planet(&self) -> &str {
            "earth"
        }
    }

    struct QueryRoot;

    #[graphql_object(scalar = MyScalarValue)]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            human {
                id
                homePlanet
            }
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"human": {"id": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }
}

mod explicit_generic_scalar {
    use std::marker::PhantomData;

    use super::*;

    struct Human<S>(PhantomData<S>);

    #[graphql_object(scalar = S)]
    impl<S: ScalarValue> Human<S> {
        fn id() -> &'static str {
            "human-32"
        }

        async fn another(&self, _executor: &Executor<'_, '_, (), S>) -> Human<S> {
            Human(PhantomData)
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human<__S>() -> Human<__S> {
            Human(PhantomData)
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            human {
                id
                another {
                    id
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"human": {
                    "id": "human-32",
                    "another": {"id": "human-32"},
                }}),
                vec![],
            )),
        );
    }
}

mod bounded_generic_scalar {
    use super::*;

    struct Human;

    #[graphql_object(scalar = S: ScalarValue + prelude::Clone)]
    impl Human {
        fn id() -> &'static str {
            "human-32"
        }

        async fn another<S>(&self, _executor: &Executor<'_, '_, (), S>) -> Human {
            Human
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            human {
                id
                another {
                    id
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"human": {
                    "id": "human-32",
                    "another": {"id": "human-32"},
                }}),
                vec![],
            )),
        );
    }
}

mod explicit_custom_context {
    use super::*;

    struct CustomContext(prelude::String);

    impl juniper::Context for CustomContext {}

    struct Human;

    #[graphql_object(context = CustomContext)]
    impl Human {
        async fn id<'c>(&self, context: &'c CustomContext) -> &'c str {
            context.0.as_str()
        }

        async fn info(_ctx: &()) -> &'static str {
            "human being"
        }

        fn more(#[graphql(context)] custom: &CustomContext) -> &str {
            custom.0.as_str()
        }
    }

    struct QueryRoot;

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            human {
                id
                info
                more
            }
        }"#;

        let schema = schema(QueryRoot);
        let ctx = CustomContext("ctx!".into());

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &ctx).await,
            Ok((
                graphql_value!({"human": {
                    "id": "ctx!",
                    "info": "human being",
                    "more": "ctx!",
                }}),
                vec![],
            )),
        );
    }
}

mod inferred_custom_context_from_field {
    use super::*;

    struct CustomContext(prelude::String);

    impl juniper::Context for CustomContext {}

    struct Human;

    #[graphql_object]
    impl Human {
        async fn id<'c>(&self, context: &'c CustomContext) -> &'c str {
            context.0.as_str()
        }

        async fn info(_ctx: &()) -> &'static str {
            "human being"
        }

        fn more(#[graphql(context)] custom: &CustomContext) -> &str {
            custom.0.as_str()
        }
    }

    struct QueryRoot;

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            human {
                id
                info
                more
            }
        }"#;

        let schema = schema(QueryRoot);
        let ctx = CustomContext("ctx!".into());

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &ctx).await,
            Ok((
                graphql_value!({"human": {
                    "id": "ctx!",
                    "info": "human being",
                    "more": "ctx!",
                }}),
                vec![],
            )),
        );
    }
}

mod executor {
    use super::*;

    struct Human;

    #[graphql_object(scalar = S: ScalarValue)]
    impl Human {
        async fn id<'e, S>(&self, executor: &'e Executor<'_, '_, (), S>) -> &'e str
        where
            S: ScalarValue,
        {
            executor.look_ahead().field_name()
        }

        fn info<S>(
            &self,
            arg: prelude::String,
            #[graphql(executor)] _another: &Executor<'_, '_, (), S>,
        ) -> prelude::String {
            arg
        }

        fn info2<'e, S>(_executor: &'e Executor<'_, '_, (), S>) -> &'e str {
            "no info"
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            human {
                id
                info(arg: "input!")
                info2
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"human": {
                    "id": "id",
                    "info": "input!",
                    "info2": "no info",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn not_arg() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                fields {
                    name
                    args {
                        name
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "id", "args": []},
                    {"name": "info", "args": [{"name": "arg"}]},
                    {"name": "info2", "args": []},
                ]}}),
                vec![],
            )),
        );
    }
}

mod switched_context {
    use super::*;

    struct CustomContext;

    impl juniper::Context for CustomContext {}

    #[derive(GraphQLObject)]
    #[graphql(context = CustomContext)]
    struct Droid {
        id: i32,
    }

    struct Human;

    #[graphql_object(context = CustomContext)]
    impl Human {
        fn switch_always<'e, S: ScalarValue>(
            executor: &'e Executor<'_, '_, CustomContext, S>,
        ) -> (&'e CustomContext, Droid) {
            (executor.context(), Droid { id: 0 })
        }

        async fn switch_opt<'e, S: ScalarValue>(
            executor: &'e Executor<'_, '_, CustomContext, S>,
        ) -> prelude::Option<(&'e CustomContext, Droid)> {
            Some((executor.context(), Droid { id: 1 }))
        }

        fn switch_res<'e, S: ScalarValue>(
            &self,
            executor: &'e Executor<'_, '_, CustomContext, S>,
        ) -> FieldResult<(&'e CustomContext, Droid)> {
            Ok((executor.context(), Droid { id: 2 }))
        }

        async fn switch_res_opt<'e, S: ScalarValue>(
            &self,
            executor: &'e Executor<'_, '_, CustomContext, S>,
        ) -> FieldResult<prelude::Option<(&'e CustomContext, Droid)>> {
            Ok(Some((executor.context(), Droid { id: 3 })))
        }
    }

    struct QueryRoot;

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn human() -> Human {
            Human
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            human {
                switchAlways { id }
                switchOpt { id }
                switchRes { id }
                switchResOpt { id }
            }
        }"#;

        let schema = schema(QueryRoot);
        let ctx = CustomContext;

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &ctx).await,
            Ok((
                graphql_value!({"human": {
                    "switchAlways": {"id": 0},
                    "switchOpt": {"id": 1},
                    "switchRes": {"id": 2},
                    "switchResOpt": {"id": 3},
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_correct_fields_types() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                fields {
                    name
                    type {
                        kind
                        name
                        ofType {
                            name
                        }
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);
        let ctx = CustomContext;

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &ctx).await,
            Ok((
                graphql_value!({"__type": {"fields": [{
                    "name": "switchAlways",
                    "type": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {"name": "Droid"},
                    },
                }, {
                    "name": "switchOpt",
                    "type": {
                        "kind": "OBJECT",
                        "name": "Droid",
                        "ofType": null,
                    },
                }, {
                    "name": "switchRes",
                    "type": {
                        "kind": "NON_NULL",
                        "name": null,
                        "ofType": {"name": "Droid"},
                    },
                }, {
                    "name": "switchResOpt",
                    "type": {
                        "kind": "OBJECT",
                        "name": "Droid",
                        "ofType": null,
                    },
                }]}}),
                vec![],
            )),
        );
    }
}
