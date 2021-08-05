//! Tests for `#[graphql_object]` macro.

use juniper::{
    execute, graphql_object, graphql_value, DefaultScalarValue, EmptyMutation, EmptySubscription,
    Executor, FieldError, GraphQLType, IntoFieldError, RootNode, ScalarValue, Variables,
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

    const DOC: &str = r#"{
        human {
            id
        }
    }"#;

    #[tokio::test]
    async fn resolves() {
        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"description": None}}), vec![])),
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

    const DOC: &str = r#"{
        human {
            id
        }
    }"#;

    #[tokio::test]
    async fn resolves() {
        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"description": None}}), vec![])),
        );
    }
}

mod raw_field {
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

    const DOC: &str = r#"{
        human {
            myId
            async
        }
    }"#;

    #[tokio::test]
    async fn resolves() {
        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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

mod fallible_field {
    use super::*;

    struct CustomError;

    impl<S: ScalarValue> IntoFieldError<S> for CustomError {
        fn into_field_error(self) -> FieldError<S> {
            juniper::FieldError::new("Whatever", graphql_value!({"code": "some"}))
        }
    }

    struct Human {
        id: String,
    }

    #[graphql_object]
    impl Human {
        fn id(&self) -> Result<&str, CustomError> {
            Ok(&self.id)
        }
    }

    #[derive(Clone, Copy)]
    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn human() -> Human {
            Human {
                id: "human-32".to_string(),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"human": {"id": "human-32"}}), vec![])),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {
                    "name": "Human",
                    "kind": "OBJECT",
                    "fields": [{
                        "name": "id",
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
    impl<B: ?Sized> Human<String, B> {
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

        fn human_string(&self) -> Human<String> {
            Human {
                id: "human-32".to_owned(),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
    impl<B: ?Sized> Human<String, B> {
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

        fn human_string(&self) -> Human<String> {
            Human {
                id: "human-32".to_owned(),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
    struct QueryRoot(String);

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

        let schema = schema(QueryRoot("mars".to_owned()));

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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

        let schema = schema(QueryRoot("mars".to_owned()));

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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

        let schema = schema(QueryRoot("mars".to_owned()));

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
    struct QueryRoot(String);

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

        let schema = schema(QueryRoot("mars".to_owned()));

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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

        let schema = schema(QueryRoot("mars".to_owned()));

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
}

mod argument {
    use super::*;

    struct Human;

    #[graphql_object]
    impl Human {
        fn id(arg: String) -> String {
            arg
        }

        fn home_planet(r#raw_arg: String) -> String {
            r#raw_arg
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"human": {"id": "human-32", "homePlanet": "earth"}}),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "id", "args": [{"name": "arg"}]},
                    {"name": "homePlanet", "args": [{"name": "rawArg"}]},
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"args": [{"description": None}]},
                    {"args": [{"description": None}]},
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"args": [{"defaultValue": None}]},
                    {"args": [{"defaultValue": None}]},
                ]}}),
                vec![],
            )),
        );
    }
}

mod default_argument {
    use super::*;

    struct Human;

    #[graphql_object]
    impl Human {
        fn id(
            #[graphql(default)] arg1: i32,
            #[graphql(default = "second".to_string())] arg2: String,
            #[graphql(default = true)] r#arg3: bool,
        ) -> String {
            format!("{}|{}&{}", arg1, arg2, r#arg3)
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
        let schema = schema(QueryRoot);

        for (input, expected) in &[
            ("{ human { id } }", "0|second&true"),
            (r#"{ human { id(arg1: 1) } }"#, "1|second&true"),
            (r#"{ human { id(arg2: "") } }"#, "0|&true"),
            (r#"{ human { id(arg1: 2, arg2: "") } }"#, "2|&true"),
            (
                r#"{ human { id(arg1: 1, arg2: "", arg3: false) } }"#,
                "1|&false",
            ),
        ] {
            let expected: &str = *expected;

            assert_eq!(
                execute(*input, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"human": {"id": expected}}), vec![])),
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
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [{"args": [
                    {"name": "arg1", "defaultValue": "0"},
                    {"name": "arg2", "defaultValue": r#""second""#},
                    {"name": "arg3", "defaultValue": "true"},
                ]}]}}),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "id", "deprecationReason": None},
                    {"name": "a", "deprecationReason": None},
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
            #[graphql(name = "myName", desc = "My argument.", default)] _n: String,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {
                    "description": "My human.",
                    "fields": [{
                        "name": "myId",
                        "description": "My human ID.",
                        "args": [{"description": "My argument."}],
                    }, {
                        "name": "a",
                        "description": None,
                        "args": [],
                    }, {
                        "name": "b",
                        "description": None,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {
                    "fields": [{
                        "name": "myId",
                        "isDeprecated": true,
                        "deprecationReason": "Not used.",
                    }, {
                        "name": "a",
                        "isDeprecated": true,
                        "deprecationReason": None,
                    }, {
                        "name": "b",
                        "isDeprecated": false,
                        "deprecationReason": None,
                    }],
                }}),
                vec![],
            )),
        );
    }
}

mod explicit_custom_context {
    use super::*;

    struct CustomContext(String);

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
            execute(DOC, None, &schema, &Variables::new(), &ctx).await,
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

    struct CustomContext(String);

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
            execute(DOC, None, &schema, &Variables::new(), &ctx).await,
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
    use juniper::LookAheadMethods as _;

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

        fn info<'b, S>(&'b self, #[graphql(executor)] _another: &Executor<'_, '_, (), S>) -> &'b str
        where
            S: ScalarValue,
        {
            "no info"
        }

        fn info2<'e, S>(_executor: &'e Executor<'_, '_, (), S>) -> &'e str
        where
            S: ScalarValue,
        {
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
                info
                info2
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"human": {
                    "id": "id",
                    "info": "no info",
                    "info2": "no info",
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"human": {"id": "human-32"}}), vec![],)),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [{"name": "id"}]}}),
                vec![],
            )),
        );
    }
}
