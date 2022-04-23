//! Tests for `#[graphql_subscription]` macro.

use std::pin::Pin;

use futures::{future, stream, FutureExt as _};
use juniper::{
    execute, graphql_object, graphql_subscription, graphql_value, graphql_vars,
    resolve_into_stream, DefaultScalarValue, EmptyMutation, Executor, FieldError, FieldResult,
    GraphQLInputObject, GraphQLType, IntoFieldError, RootNode, ScalarValue,
};

use crate::util::extract_next;

struct Query;

#[graphql_object]
impl Query {
    fn empty() -> bool {
        true
    }
}

fn schema<'q, C, Qry, Sub>(
    query_root: Qry,
    subscription_root: Sub,
) -> RootNode<'q, Qry, EmptyMutation<C>, Sub>
where
    Qry: GraphQLType<DefaultScalarValue, Context = C, TypeInfo = ()> + 'q,
    Sub: GraphQLType<DefaultScalarValue, Context = C, TypeInfo = ()> + 'q,
{
    RootNode::new(query_root, EmptyMutation::<C>::new(), subscription_root)
}

fn schema_with_scalar<'q, S, C, Qry, Sub>(
    query_root: Qry,
    subscription_root: Sub,
) -> RootNode<'q, Qry, EmptyMutation<C>, Sub, S>
where
    Qry: GraphQLType<S, Context = C, TypeInfo = ()> + 'q,
    Sub: GraphQLType<S, Context = C, TypeInfo = ()> + 'q,
    S: ScalarValue + 'q,
{
    RootNode::new_with_scalar_value(query_root, EmptyMutation::<C>::new(), subscription_root)
}

type Stream<'a, I> = Pin<Box<dyn futures::Stream<Item = I> + Send + 'a>>;

mod trivial {
    use super::*;

    struct Human;

    #[graphql_subscription]
    impl Human {
        async fn id() -> Stream<'static, String> {
            Box::pin(stream::once(future::ready("human-32".to_owned())))
        }

        // TODO: Make work for `Stream<'_, String>`.
        async fn home_planet(&self) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready("earth".to_owned())))
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "human-32"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_home_planet_field() {
        const DOC: &str = r#"subscription {
            homePlanet
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"homePlanet": "earth"}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_graphql_object() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                kind
            }
        }"#;

        let schema = schema(Query, Human);

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

        let schema = schema(Query, Human);

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

        let schema = schema(Query, Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"description": null}}), vec![])),
        );
    }
}

mod raw_method {
    use super::*;

    struct Human;

    #[graphql_subscription]
    impl Human {
        async fn r#my_id() -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("human-32")))
        }

        async fn r#async(&self) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("async-32")))
        }
    }

    #[tokio::test]
    async fn resolves_my_id_field() {
        const DOC: &str = r#"subscription {
            myId
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"myId": "human-32"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_async_field() {
        const DOC: &str = r#"subscription {
            async
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"async": "async-32"}), vec![])),
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

        let schema = schema(Query, Human);

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

    #[graphql_subscription]
    impl Human {
        async fn id() -> Stream<'static, String> {
            Box::pin(stream::once(future::ready("human-32".to_owned())))
        }

        #[allow(dead_code)]
        #[graphql(ignore)]
        fn planet() -> &'static str {
            "earth"
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "human-32"}), vec![])),
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

        let schema = schema(Query, Human);

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
            juniper::FieldError::new("Whatever", graphql_value!({"code": "some"}))
        }
    }

    struct Human;

    #[graphql_subscription]
    impl Human {
        async fn id(&self) -> Result<Stream<'static, String>, CustomError> {
            Ok(Box::pin(stream::once(future::ready("human-32".to_owned()))))
        }

        async fn home_planet<__S>() -> FieldResult<Stream<'static, &'static str>, __S> {
            Ok(Box::pin(stream::once(future::ready("earth"))))
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "human-32"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_home_planet_field() {
        const DOC: &str = r#"subscription {
            homePlanet
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"homePlanet": "earth"}), vec![])),
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

        let schema = schema(Query, Human);

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

mod argument {
    use super::*;

    struct Human;

    #[graphql_subscription]
    impl Human {
        async fn id(arg: String) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready(arg)))
        }

        async fn home_planet(
            &self,
            r#raw_arg: String,
            r#async: Option<i32>,
        ) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready(format!(
                "{},{:?}",
                r#raw_arg, r#async
            ))))
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"subscription {
            id(arg: "human-32")
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "human-32"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_home_planet_field() {
        const DOC: &str = r#"subscription {
            homePlanet(rawArg: "earth")
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"homePlanet": "earth,None"}), vec![])),
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

        let schema = schema(Query, Human);

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

        let schema = schema(Query, Human);

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

        let schema = schema(Query, Human);

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

    #[graphql_subscription]
    impl Human {
        async fn id(
            &self,
            #[graphql(default)] arg1: i32,
            #[graphql(default = "second".to_string())] arg2: String,
            #[graphql(default = true)] r#arg3: bool,
        ) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready(format!(
                "{}|{}&{}",
                arg1, arg2, r#arg3
            ))))
        }

        async fn info(#[graphql(default = Point { x: 1 })] coord: Point) -> Stream<'static, i32> {
            Box::pin(stream::once(future::ready(coord.x)))
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        let schema = schema(Query, Human);

        for (input, expected) in &[
            ("subscription { id }", "0|second&true"),
            ("subscription { id(arg1: 1) }", "1|second&true"),
            (r#"subscription { id(arg2: "") }"#, "0|&true"),
            (r#"subscription { id(arg1: 2, arg2: "") }"#, "2|&true"),
            (
                r#"subscription { id(arg1: 1, arg2: "", arg3: false) }"#,
                "1|&false",
            ),
        ] {
            let expected: &str = *expected;

            assert_eq!(
                resolve_into_stream(*input, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({ "id": expected }), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn resolves_info_field() {
        let schema = schema(Query, Human);

        for (input, expected) in &[
            ("subscription { info }", 1),
            ("subscription { info(coord: { x: 2 }) }", 2),
        ] {
            let expected: i32 = *expected;

            assert_eq!(
                resolve_into_stream(*input, None, &schema, &graphql_vars! {}, &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({ "info": expected }), vec![])),
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

        let schema = schema(Query, Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [{
                    "args": [{
                        "name": "arg1",
                        "defaultValue": "0",
                        "type": {"name": "Int", "ofType": null},
                    }, {
                        "name": "arg2",
                        "defaultValue": r#""second""#,
                        "type": {"name": "String", "ofType": null},
                    }, {
                        "name": "arg3",
                        "defaultValue": "true",
                        "type": {"name": "Boolean", "ofType": null},
                    }],
                }, {
                    "args": [{
                        "name": "coord",
                        "defaultValue": "{x: 1}",
                        "type": {"name": "Point", "ofType": null},
                    }],
                }]}}),
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

    #[graphql_subscription]
    impl<B: ?Sized + Sync> Human<i32, B> {
        async fn id(&self) -> Stream<'static, i32> {
            Box::pin(stream::once(future::ready(self.id)))
        }
    }

    #[graphql_subscription(name = "HumanString")]
    impl<B: ?Sized + Sync> Human<String, B> {
        async fn id(&self) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready(self.id.to_owned())))
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(
            Query,
            Human {
                id: 34i32,
                _home_planet: (),
            },
        );

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": 34}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_human_string() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(
            Query,
            Human {
                id: "human-32".to_owned(),
                _home_planet: (),
            },
        );

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "human-32"}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_type_name_without_type_params() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                name
            }
        }"#;

        let schema = schema(
            Query,
            Human {
                id: 0i32,
                _home_planet: (),
            },
        );

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Human"}}), vec![])),
        );
    }
}

mod generic_lifetime {
    use super::*;

    struct Human<'p, A = ()> {
        id: A,
        home_planet: &'p str,
    }

    #[graphql_subscription]
    impl<'p> Human<'p, i32> {
        async fn id(&self) -> Stream<'static, i32> {
            Box::pin(stream::once(future::ready(self.id)))
        }

        // TODO: Make it work with `Stream<'_, &str>`.
        async fn planet(&self) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready(self.home_planet.to_owned())))
        }
    }

    #[graphql_subscription(name = "HumanString")]
    impl<'id, 'p> Human<'p, &'id str> {
        // TODO: Make it work with `Stream<'_, &str>`.
        async fn id(&self) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready(self.id.to_owned())))
        }

        // TODO: Make it work with `Stream<'_, &str>`.
        async fn planet(&self) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready(self.home_planet.to_owned())))
        }
    }

    #[tokio::test]
    async fn resolves_human_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(
            Query,
            Human {
                id: 34i32,
                home_planet: "earth",
            },
        );

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": 34}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_human_planet_field() {
        const DOC: &str = r#"subscription {
            planet
        }"#;

        let schema = schema(
            Query,
            Human {
                id: 34i32,
                home_planet: "earth",
            },
        );

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"planet": "earth"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_human_string_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(
            Query,
            Human {
                id: "human-32",
                home_planet: "mars",
            },
        );

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "human-32"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_human_string_planet_field() {
        const DOC: &str = r#"subscription {
            planet
        }"#;

        let schema = schema(
            Query,
            Human {
                id: "human-32",
                home_planet: "mars",
            },
        );

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"planet": "mars"}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_type_name_without_type_params() {
        const DOC: &str = r#"{
            __type(name: "Human") {
                name
            }
        }"#;

        let schema = schema(
            Query,
            Human {
                id: 34i32,
                home_planet: "earth",
            },
        );

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Human"}}), vec![])),
        );
    }
}

mod description_from_doc_comment {
    use super::*;

    struct Human;

    /// Rust docs.
    #[graphql_subscription]
    impl Human {
        /// Rust `id` docs.
        /// Here.
        async fn id() -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("human-32")))
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

        let schema = schema(Query, Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "description": "Rust docs.",
                    "fields": [{"description": "Rust `id` docs.\nHere."}],
                }}),
                vec![],
            )),
        );
    }
}

mod deprecation_from_attr {
    use super::*;

    struct Human;

    #[graphql_subscription]
    impl Human {
        async fn id() -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("human-32")))
        }

        #[deprecated]
        async fn a(&self) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("a")))
        }

        #[deprecated(note = "Use `id`.")]
        async fn b(&self) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("b")))
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "human-32"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_deprecated_a_field() {
        const DOC: &str = r#"subscription {
            a
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"a": "a"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_deprecated_b_field() {
        const DOC: &str = r#"subscription {
            b
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"b": "b"}), vec![])),
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

        let schema = schema(Query, Human);

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

        let schema = schema(Query, Human);

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
    #[graphql_subscription(name = "MyHuman", desc = "My human.")]
    impl Human {
        /// Rust `id` docs.
        #[graphql(name = "myId", desc = "My human ID.", deprecated = "Not used.")]
        #[deprecated(note = "Should be omitted.")]
        async fn id(
            #[graphql(name = "myName", desc = "My argument.", default)] _n: String,
        ) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("human-32")))
        }

        #[graphql(deprecated)]
        #[deprecated(note = "Should be omitted.")]
        async fn a(&self) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("a")))
        }

        async fn b(&self) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("b")))
        }
    }

    #[tokio::test]
    async fn resolves_deprecated_id_field() {
        const DOC: &str = r#"subscription {
            myId
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"myId": "human-32"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_deprecated_a_field() {
        const DOC: &str = r#"subscription {
            a
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"a": "a"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_b_field() {
        const DOC: &str = r#"subscription {
            b
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"b": "b"}), vec![])),
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

        let schema = schema(Query, Human);

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

        let schema = schema(Query, Human);

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

        let schema = schema(Query, Human);

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

    #[graphql_subscription(rename_all = "none")]
    impl Human {
        async fn id() -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("human-32")))
        }

        async fn home_planet(&self, planet_name: String) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready(planet_name)))
        }

        async fn r#async_info(r#my_num: i32) -> Stream<'static, i32> {
            Box::pin(stream::once(future::ready(r#my_num)))
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "human-32"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_home_planet_field() {
        const DOC: &str = r#"subscription {
            home_planet(planet_name: "earth")
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"home_planet": "earth"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_async_info_field() {
        const DOC: &str = r#"subscription {
            async_info(my_num: 3)
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"async_info": 3}), vec![])),
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

        let schema = schema(Query, Human);

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

    #[graphql_subscription(scalar = DefaultScalarValue)]
    impl Human {
        async fn id(&self) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("human-32")))
        }

        async fn home_planet() -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("earth")))
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "human-32"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_home_planet_field() {
        const DOC: &str = r#"subscription {
            homePlanet
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"homePlanet": "earth"}), vec![])),
        );
    }
}

mod custom_scalar {
    use crate::custom_scalar::MyScalarValue;

    use super::*;

    struct Human;

    #[graphql_subscription(scalar = MyScalarValue)]
    impl Human {
        async fn id(&self) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("human-32")))
        }

        async fn home_planet() -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("earth")))
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _, _>(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "human-32"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_home_planet_field() {
        const DOC: &str = r#"subscription {
            homePlanet
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _, _>(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"homePlanet": "earth"}), vec![])),
        );
    }
}

mod explicit_generic_scalar {
    use std::marker::PhantomData;

    use super::*;

    struct Human<S>(PhantomData<S>);

    #[graphql_subscription(scalar = S)]
    impl<S: ScalarValue> Human<S> {
        async fn id(&self) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("human-32")))
        }

        async fn home_planet(_executor: &Executor<'_, '_, (), S>) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("earth")))
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(Query, Human::<DefaultScalarValue>(PhantomData));

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "human-32"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_home_planet_field() {
        const DOC: &str = r#"subscription {
            homePlanet
        }"#;

        let schema = schema(Query, Human::<DefaultScalarValue>(PhantomData));

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"homePlanet": "earth"}), vec![])),
        );
    }
}

mod bounded_generic_scalar {
    use super::*;

    struct Human;

    #[graphql_subscription(scalar = S: ScalarValue + Clone)]
    impl Human {
        async fn id(&self) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("human-32")))
        }

        async fn home_planet<S>(
            _executor: &Executor<'_, '_, (), S>,
        ) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("earth")))
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "human-32"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_home_planet_field() {
        const DOC: &str = r#"subscription {
            homePlanet
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"homePlanet": "earth"}), vec![])),
        );
    }
}

mod explicit_custom_context {
    use super::*;

    struct CustomContext(String);

    impl juniper::Context for CustomContext {}

    struct QueryRoot;

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn empty() -> bool {
            true
        }
    }

    struct Human;

    #[graphql_subscription(context = CustomContext)]
    impl Human {
        // TODO: Make work for `Stream<'c, String>`.
        async fn id<'c>(&self, context: &'c CustomContext) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready(context.0.clone())))
        }

        // TODO: Make work for `Stream<'_, String>`.
        async fn info(_ctx: &()) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("human being")))
        }

        async fn more(#[graphql(context)] custom: &CustomContext) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready(custom.0.clone())))
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(QueryRoot, Human);
        let ctx = CustomContext("ctx!".into());

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &ctx)
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "ctx!"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_info_field() {
        const DOC: &str = r#"subscription {
            info
        }"#;

        let schema = schema(QueryRoot, Human);
        let ctx = CustomContext("ctx!".into());

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &ctx)
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"info": "human being"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_more_field() {
        const DOC: &str = r#"subscription {
            more
        }"#;

        let schema = schema(QueryRoot, Human);
        let ctx = CustomContext("ctx!".into());

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &ctx)
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"more": "ctx!"}), vec![])),
        );
    }
}

mod inferred_custom_context_from_field {
    use super::*;

    struct CustomContext(String);

    impl juniper::Context for CustomContext {}

    struct QueryRoot;

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn empty() -> bool {
            true
        }
    }

    struct Human;

    #[graphql_subscription]
    impl Human {
        // TODO: Make work for `Stream<'c, String>`.
        async fn id<'c>(&self, context: &'c CustomContext) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready(context.0.clone())))
        }

        // TODO: Make work for `Stream<'_, String>`.
        async fn info(_ctx: &()) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("human being")))
        }

        async fn more(#[graphql(context)] custom: &CustomContext) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready(custom.0.clone())))
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(QueryRoot, Human);
        let ctx = CustomContext("ctx!".into());

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &ctx)
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "ctx!"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_info_field() {
        const DOC: &str = r#"subscription {
            info
        }"#;

        let schema = schema(QueryRoot, Human);
        let ctx = CustomContext("ctx!".into());

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &ctx)
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"info": "human being"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_more_field() {
        const DOC: &str = r#"subscription {
            more
        }"#;

        let schema = schema(QueryRoot, Human);
        let ctx = CustomContext("ctx!".into());

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &ctx)
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"more": "ctx!"}), vec![])),
        );
    }
}

mod executor {
    use juniper::LookAheadMethods as _;

    use super::*;

    struct Human;

    #[graphql_subscription(scalar = S: ScalarValue)]
    impl Human {
        // TODO: Make work for `Stream<'e, &'e str>`.
        async fn id<'e, S>(&self, executor: &'e Executor<'_, '_, (), S>) -> Stream<'static, String>
        where
            S: ScalarValue,
        {
            Box::pin(stream::once(future::ready(
                executor.look_ahead().field_name().to_owned(),
            )))
        }

        async fn info<S>(
            &self,
            arg: String,
            #[graphql(executor)] _another: &Executor<'_, '_, (), S>,
        ) -> Stream<'static, String> {
            Box::pin(stream::once(future::ready(arg)))
        }

        // TODO: Make work for `Stream<'e, &'e str>`.
        async fn info2<'e, S>(
            _executor: &'e Executor<'_, '_, (), S>,
        ) -> Stream<'static, &'static str> {
            Box::pin(stream::once(future::ready("no info")))
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"subscription {
            id
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"id": "id"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_info_field() {
        const DOC: &str = r#"subscription {
            info(arg: "input!")
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"info": "input!"}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_info2_field() {
        const DOC: &str = r#"subscription {
            info2
        }"#;

        let schema = schema(Query, Human);

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &graphql_vars! {}, &())
                .then(|s| extract_next(s))
                .await,
            Ok((graphql_value!({"info2": "no info"}), vec![])),
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

        let schema = schema(Query, Human);

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

    #[tokio::test]
    async fn test_integration_json() {
        use juniper::integrations::json::{TypedJson, TypedJsonInfo};
        use serde_json::json;

        struct Foo;
        impl TypedJsonInfo for Foo {
            fn type_name() -> &'static str {
                "Foo"
            }
            fn schema() -> &'static str {
                r#"
                type Foo {
                    message: [String]
                }
                "#
            }
        }

        struct Query;
        #[graphql_object]
        impl Query {
            fn zero() -> FieldResult<i32> {
                Ok(0)
            }
        }

        struct Subscription;
        #[graphql_subscription(scalar = S: ScalarValue)]
        impl Subscription {
            // TODO: Make work for `Stream<'e, &'e str>`.
            async fn foo<'e, S>(
                &self,
                _executor: &'e Executor<'_, '_, (), S>,
            ) -> Stream<'static, TypedJson<Foo>>
            where
                S: ScalarValue,
            {
                let data = TypedJson::new(json!({"message": ["Hello World"] }));
                Box::pin(stream::once(future::ready(data)))
            }
        }

        let schema = juniper::RootNode::new(Query, EmptyMutation::new(), Subscription);

        const DOC: &str = r#"subscription {
            foo { message }
        }"#;

        assert_eq!(
            resolve_into_stream(DOC, None, &schema, &Variables::new(), &())
                .then(|s| extract_next(s))
                .await,
            Ok((
                graphql_value!({"foo":{"message": ["Hello World"] }}),
                vec![]
            )),
        );
    }
}
