//! Tests for `#[derive(GraphQLObject)]` macro.

pub mod common;

use juniper::{
    execute, graphql_object, graphql_value, graphql_vars, DefaultScalarValue, GraphQLObject,
    ScalarValue,
};

use self::common::util::{schema, schema_with_scalar};

// Override `std::prelude` items to check whether macros expand hygienically.
#[allow(unused_imports)]
use self::common::hygiene::*;

mod trivial {
    use super::*;

    #[derive(GraphQLObject)]
    struct Human {
        id: &'static str,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human { id: "human-32" }
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

mod raw_field {
    use super::*;

    #[derive(GraphQLObject)]
    struct Human {
        r#async: &'static str,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human {
                r#async: "human-32",
            }
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            human {
                async
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"human": {"async": "human-32"}}), vec![])),
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
                    "fields": [{"name": "async"}],
                }}),
                vec![],
            )),
        );
    }
}

mod ignored_field {
    use super::*;

    #[derive(GraphQLObject)]
    struct Human {
        id: &'static str,
        #[allow(dead_code)]
        #[graphql(ignore)]
        planet: &'static str,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human {
                id: "human-32",
                planet: "earth",
            }
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

mod generic {
    use super::*;

    #[derive(GraphQLObject)]
    struct Human<B: ?Sized = ()> {
        id: &'static str,
        #[graphql(ignore)]
        _home_planet: B,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human {
                id: "human-32",
                _home_planet: (),
            }
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

mod generic_lifetime {
    use super::*;

    #[derive(GraphQLObject)]
    struct Human<'id, B: ?Sized = ()> {
        id: &'id str,
        #[graphql(ignore)]
        _home_planet: B,
    }

    struct QueryRoot(prelude::String);

    #[graphql_object]
    impl QueryRoot {
        fn human(&self) -> Human<'_, i32> {
            Human {
                id: self.0.as_str(),
                _home_planet: 32,
            }
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            human {
                id
            }
        }"#;

        let schema = schema(QueryRoot("mars".into()));

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"human": {"id": "mars"}}), vec![])),
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

    #[derive(GraphQLObject)]
    struct Droid<'p, A = ()> {
        #[graphql(ignore)]
        _id: A,
        primary_function: &'p str,
    }

    #[derive(GraphQLObject)]
    struct Human<'d, A: prelude::Sync = ()> {
        id: i32,
        droid: Droid<'d, A>,
    }

    struct QueryRoot(prelude::String);

    #[graphql_object]
    impl QueryRoot {
        fn human(&self) -> Human<'_, i8> {
            Human {
                id: 32,
                droid: Droid {
                    _id: 12,
                    primary_function: self.0.as_str(),
                },
            }
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            human {
                id
                droid {
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
                    "droid": {
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

mod description_from_doc_comment {
    use super::*;

    /// Rust docs.\
    /// Here.
    #[derive(GraphQLObject)]
    struct Human {
        /// Rust `id` docs.
        /// Here.
        id: prelude::String,
    }

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
                    "description": "Rust docs. Here.",
                    "fields": [{"description": "Rust `id` docs.\nHere."}],
                }}),
                vec![],
            )),
        );
    }
}

mod deprecation_from_attr {
    use super::*;

    #[derive(GraphQLObject)]
    struct Human {
        id: prelude::String,
        #[deprecated]
        a: &'static str,
        #[deprecated(note = "Use `id`.")]
        b: &'static str,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        #[allow(deprecated)]
        fn human() -> Human {
            Human {
                id: "human-32".into(),
                a: "a",
                b: "b",
            }
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

    /// Rust docs.
    #[derive(GraphQLObject)]
    #[graphql(name = "MyHuman", desc = "My human.")]
    struct Human {
        /// Rust `id` docs.
        #[graphql(name = "myId", desc = "My human ID.", deprecated = "Not used.")]
        #[deprecated(note = "Should be omitted.")]
        id: prelude::String,
        #[graphql(deprecated)]
        #[deprecated(note = "Should be omitted.")]
        a: &'static str,
        b: &'static str,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        #[allow(deprecated)]
        fn human() -> Human {
            Human {
                id: "human-32".into(),
                a: "a",
                b: "b",
            }
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
                        {"name": "myId"},
                        {"name": "a"},
                        {"name": "b"},
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
                    }, {
                        "name": "a",
                        "description": null,
                    }, {
                        "name": "b",
                        "description": null,
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

mod renamed_all_fields {
    use super::*;

    #[derive(GraphQLObject)]
    #[graphql(rename_all = "none")]
    struct Human {
        id: &'static str,
        home_planet: prelude::String,
        r#async_info: i32,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human {
                id: "human-32",
                home_planet: "earth".into(),
                r#async_info: 3,
            }
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            human {
                id
                home_planet
                async_info
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
    async fn uses_correct_fields_names() {
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
                graphql_value!({"__type": {"fields": [
                    {"name": "id"},
                    {"name": "home_planet"},
                    {"name": "async_info"},
                ]}}),
                vec![],
            )),
        );
    }
}

mod explicit_scalar {
    use super::*;

    #[derive(GraphQLObject)]
    #[graphql(scalar = DefaultScalarValue)]
    struct Human {
        id: &'static str,
    }

    struct QueryRoot;

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn human() -> Human {
            Human { id: "human-32" }
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
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
}

mod custom_scalar {
    use crate::common::MyScalarValue;

    use super::*;

    #[derive(GraphQLObject)]
    #[graphql(scalar = MyScalarValue)]
    struct Human {
        id: &'static str,
    }

    struct QueryRoot;

    #[graphql_object(scalar = MyScalarValue)]
    impl QueryRoot {
        fn human() -> Human {
            Human { id: "human-32" }
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            human {
                id
            }
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"human": {"id": "human-32"}}), vec![])),
        );
    }
}

mod explicit_generic_scalar {
    use std::marker::PhantomData;

    use super::*;

    #[derive(GraphQLObject)]
    #[graphql(scalar = S)]
    struct Human<S: prelude::Clone> {
        id: &'static str,
        #[graphql(ignore)]
        _scalar: PhantomData<S>,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human<__S: prelude::Clone>() -> Human<__S> {
            Human {
                id: "human-32",
                _scalar: PhantomData,
            }
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
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
}

mod bounded_generic_scalar {
    use super::*;

    #[derive(GraphQLObject)]
    #[graphql(scalar = S: ScalarValue + prelude::Clone)]
    struct Human {
        id: &'static str,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human { id: "human-32" }
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
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
}

mod explicit_custom_context {
    use super::*;

    struct CustomContext(prelude::String);

    impl juniper::Context for CustomContext {}

    #[derive(GraphQLObject)]
    #[graphql(context = CustomContext)]
    struct Human<'s> {
        id: &'s str,
    }

    struct QueryRoot;

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn human(ctx: &CustomContext) -> Human<'_> {
            Human { id: ctx.0.as_str() }
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            human {
                id
            }
        }"#;

        let schema = schema(QueryRoot);
        let ctx = CustomContext("ctx!".into());

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &ctx).await,
            Ok((graphql_value!({"human": {"id": "ctx!"}}), vec![])),
        );
    }
}
