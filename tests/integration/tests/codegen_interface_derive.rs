//! Tests for `#[derive(GraphQLInterface)]` macro.

pub mod common;

use juniper::{
    execute, graphql_object, graphql_value, graphql_vars, DefaultScalarValue, FieldError,
    FieldResult, GraphQLInterface, GraphQLObject, GraphQLUnion, IntoFieldError, ScalarValue, ID,
};

use self::common::util::{schema, schema_with_scalar};

// Override `std::prelude` items to check whether macros expand hygienically.
#[allow(unused_imports)]
use self::common::hygiene::*;

mod no_implers {
    use super::*;

    #[derive(GraphQLInterface)]
    struct Character {
        id: prelude::String,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_struct_name() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
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

mod trivial {
    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = [Human, Droid])]
    struct Character {
        id: prelude::String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    struct Droid {
        id: prelude::String,
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        async fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "human-32",
                    "homePlanet": "earth",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "droidId": "droid-99",
                    "primaryFunction": "run",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in [
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn registers_all_implementers() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                possibleTypes {
                    kind
                    name
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"possibleTypes": [
                    {"kind": "OBJECT", "name": "Droid"},
                    {"kind": "OBJECT", "name": "Human"},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn registers_itself_in_implementers() {
        let schema = schema(QueryRoot::Human);

        for object in ["Human", "Droid"] {
            let doc = format!(
                r#"{{
                   __type(name: "{object}") {{
                       interfaces {{
                           kind
                           name
                       }}
                   }}
                }}"#,
            );

            assert_eq!(
                execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
                Ok((
                    graphql_value!({"__type": {"interfaces": [
                        {"kind": "INTERFACE", "name": "Character"},
                    ]}}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn uses_struct_name() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                description
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"description": null}}), vec![])),
        );
    }
}

mod explicit_alias {
    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(enum = CharacterEnum, for = [Human, Droid])]
    struct Character {
        id: prelude::String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterEnum)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    struct Droid {
        id: prelude::String,
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterEnum)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterEnum {
            match self {
                Self::Human => Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "human-32",
                    "homePlanet": "earth",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "droidId": "droid-99",
                    "primaryFunction": "run",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in [
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_struct_name() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                description
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"description": null}}), vec![])),
        );
    }
}

mod trivial_async {
    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = [Human, Droid])]
    struct Character {
        id: prelude::String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    struct Droid {
        id: prelude::String,
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        async fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "human-32",
                    "homePlanet": "earth",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "droidId": "droid-99",
                    "primaryFunction": "run",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in [
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn registers_all_implementers() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                possibleTypes {
                    kind
                    name
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"possibleTypes": [
                    {"kind": "OBJECT", "name": "Droid"},
                    {"kind": "OBJECT", "name": "Human"},
                ]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn registers_itself_in_implementers() {
        let schema = schema(QueryRoot::Human);

        for object in ["Human", "Droid"] {
            let doc = format!(
                r#"{{
                   __type(name: "{object}") {{
                       interfaces {{
                           kind
                           name
                       }}
                   }}
                }}"#,
            );

            assert_eq!(
                execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
                Ok((
                    graphql_value!({"__type": {"interfaces": [
                        {"kind": "INTERFACE", "name": "Character"},
                    ]}}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn uses_struct_name() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                description
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"description": null}}), vec![])),
        );
    }
}

mod fallible_field {
    use super::*;

    struct CustomError;

    impl<S: ScalarValue> IntoFieldError<S> for CustomError {
        fn into_field_error(self) -> FieldError<S> {
            FieldError::new("Whatever", graphql_value!({"code": "some"}))
        }
    }

    #[derive(GraphQLInterface)]
    #[graphql(for = [Human, Droid])]
    struct Character {
        id: prelude::Result<prelude::String, CustomError>,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    struct Droid {
        id: prelude::String,
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        fn id(&self) -> prelude::Result<prelude::String, CustomError> {
            Ok(self.id.clone())
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "human-32",
                    "homePlanet": "earth",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "droidId": "droid-99",
                    "primaryFunction": "run",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in [
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn has_correct_graphql_type() {
        const DOC: &str = r#"{
            __type(name: "Character") {
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

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {
                    "name": "Character",
                    "kind": "INTERFACE",
                    "fields": [{
                        "name": "id",
                        "type": {
                            "kind": "NON_NULL",
                            "ofType": {"name": "String"},
                        },
                    }],
                }}),
                vec![],
            )),
        );
    }
}

mod generic {
    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = [Human, Droid])]
    struct Character<A = (), B: ?Sized = ()> {
        id: prelude::String,

        #[graphql(skip)]
        _phantom: std::marker::PhantomData<(A, B)>,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    struct Droid {
        id: prelude::String,
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterValue<(), u8>)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "human-32",
                    "homePlanet": "earth",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "droidId": "droid-99",
                    "primaryFunction": "run",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in [
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn uses_struct_name_without_type_params() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                name
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }
}

mod description_from_doc_comment {
    use super::*;

    /// Rust docs.
    #[derive(GraphQLInterface)]
    #[graphql(for = Human)]
    struct Character {
        /// Rust `id` docs.
        /// Long.
        id: prelude::String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".into(),
                home_planet: "earth".into(),
            }
            .into()
        }
    }

    #[tokio::test]
    async fn uses_doc_comment_as_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
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
                    "fields": [{"description": "Rust `id` docs.\nLong."}],
                }}),
                vec![],
            )),
        );
    }
}

mod deprecation_from_attr {
    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = Human)]
    struct Character {
        id: prelude::String,

        #[deprecated]
        a: prelude::String,

        #[deprecated(note = "Use `id`.")]
        b: prelude::String,
    }

    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Human {
        fn id(&self) -> &str {
            &self.id
        }

        fn human_planet(&self) -> &str {
            &self.home_planet
        }

        fn a() -> &'static str {
            "a"
        }

        fn b() -> prelude::String {
            "b".into()
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".into(),
                home_planet: "earth".into(),
            }
            .into()
        }
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"character": {"id": "human-32"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_deprecated_fields() {
        const DOC: &str = r#"{
            character {
                a
                b
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"character": {"a": "a", "b": "b"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn deprecates_fields() {
        const DOC: &str = r#"{
            __type(name: "Character") {
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
            __type(name: "Character") {
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
    #[derive(GraphQLInterface)]
    #[graphql(name = "MyChar", desc = "My character.", for = Human)]
    struct Character {
        /// Rust `id` docs.
        #[graphql(name = "myId", desc = "My character ID.", deprecated = "Not used.")]
        #[deprecated(note = "Should be omitted.")]
        id: prelude::String,

        #[graphql(deprecated)]
        #[deprecated(note = "Should be omitted.")]
        a: prelude::String,

        b: prelude::String,
    }

    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Human {
        fn my_id(&self, #[graphql(name = "myName")] _: prelude::Option<prelude::String>) -> &str {
            &self.id
        }

        fn home_planet(&self) -> &str {
            &self.home_planet
        }

        fn a() -> prelude::String {
            "a".into()
        }

        fn b() -> &'static str {
            "b"
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".into(),
                home_planet: "earth".into(),
            }
            .into()
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            character {
                myId
                a
                b
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "myId": "human-32",
                    "a": "a",
                    "b": "b",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_custom_name() {
        const DOC: &str = r#"{
            __type(name: "MyChar") {
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
                    "name": "MyChar",
                    "fields": [
                        {"name": "myId", "args": []},
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
            __type(name: "MyChar") {
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
                    "description": "My character.",
                    "fields": [{
                        "name": "myId",
                        "description": "My character ID.",
                        "args": [],
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
            __type(name: "MyChar") {
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

    #[derive(GraphQLInterface)]
    #[graphql(rename_all = "none", for = Human)]
    struct Character {
        id: prelude::String,
    }

    struct Human;

    #[graphql_object(rename_all = "none", impl = CharacterValue)]
    impl Human {
        fn id() -> &'static str {
            "human-32"
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human.into()
        }
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "id": "human-32",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_correct_fields_and_args_names() {
        const DOC: &str = r#"{
            __type(name: "Character") {
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
                ]}}),
                vec![],
            )),
        );
    }
}

mod explicit_scalar {
    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = [Human, Droid])]
    #[graphql(scalar = DefaultScalarValue)]
    struct Character {
        id: prelude::String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue, scalar = DefaultScalarValue)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    struct Droid {
        id: prelude::String,
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterValue, scalar = DefaultScalarValue)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "human-32",
                    "homePlanet": "earth",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "droidId": "droid-99",
                    "primaryFunction": "run",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in [
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod custom_scalar {
    use crate::common::MyScalarValue;

    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = [Human, Droid], scalar = MyScalarValue)]
    struct Character {
        id: prelude::String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue, scalar = MyScalarValue)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    struct Droid {
        id: prelude::String,
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterValue, scalar = MyScalarValue)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = MyScalarValue)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "human-32",
                    "homePlanet": "earth",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "droidId": "droid-99",
                    "primaryFunction": "run",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in [
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema_with_scalar::<MyScalarValue, _, _>(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod explicit_generic_scalar {
    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = [Human, Droid], scalar = S)]
    struct Character<S: ScalarValue = DefaultScalarValue> {
        id: FieldResult<prelude::String, S>,
    }

    #[derive(GraphQLObject)]
    #[graphql(scalar = S: ScalarValue, impl = CharacterValue<S>)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    struct Droid {
        id: prelude::String,
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterValue<__S>)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = S: ScalarValue)]
    impl QueryRoot {
        fn character<S: ScalarValue>(&self) -> CharacterValue<S> {
            match self {
                Self::Human => Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "human-32",
                    "homePlanet": "earth",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "droidId": "droid-99",
                    "primaryFunction": "run",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in [
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod bounded_generic_scalar {
    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = [Human, Droid], scalar = S: ScalarValue + prelude::Clone)]
    struct Character {
        id: prelude::String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue, scalar = S: ScalarValue + prelude::Clone)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    struct Droid {
        id: prelude::String,
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterValue, scalar = S: ScalarValue + prelude::Clone)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "human-32",
                    "homePlanet": "earth",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "droidId": "droid-99",
                    "primaryFunction": "run",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in [
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod ignored_method {
    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = Human)]
    struct Character {
        id: prelude::String,

        #[graphql(ignore)]
        ignored: prelude::Option<Human>,

        #[graphql(skip)]
        skipped: i32,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".into(),
                home_planet: "earth".into(),
            }
            .into()
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "human-32",
                    "homePlanet": "earth",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"character": {"id": "human-32"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_not_field() {
        const DOC: &str = r#"{
            __type(name: "Character") {
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

mod field_return_subtyping {
    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = [Human, Droid])]
    struct Character {
        id: prelude::Option<prelude::String>,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    struct Droid {
        id: prelude::String,
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "human-32",
                    "homePlanet": "earth",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "droidId": "droid-99",
                    "primaryFunction": "run",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in [
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod field_return_union_subtyping {
    use super::*;

    #[derive(GraphQLObject)]
    struct Strength {
        value: i32,
    }

    #[derive(GraphQLObject)]
    struct Knowledge {
        value: i32,
    }

    #[allow(dead_code)]
    #[derive(GraphQLUnion)]
    enum KeyFeature {
        Strength(Strength),
        Knowledge(Knowledge),
    }

    #[derive(GraphQLInterface)]
    #[graphql(for = [Human, Droid])]
    struct Character {
        id: prelude::Option<prelude::String>,
        key_feature: KeyFeature,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
        key_feature: Knowledge,
    }

    struct Droid {
        id: prelude::String,
        primary_function: prelude::String,
        strength: i32,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        fn id(&self) -> &str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }

        fn key_feature(&self) -> Strength {
            Strength {
                value: self.strength,
            }
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                    key_feature: Knowledge { value: 10 },
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                    strength: 42,
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                    keyFeature {
                        value
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "human-32",
                    "homePlanet": "earth",
                    "keyFeature": {"value": 10},
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id
                    primaryFunction
                    keyFeature {
                        value
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "droidId": "droid-99",
                    "primaryFunction": "run",
                    "keyFeature": {"value": 42},
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_fields() {
        const DOC: &str = r#"{
            character {
                id
                keyFeature {
                    ...on Strength {
                        value
                    }
                    ... on Knowledge {
                        value
                    }
                }
            }
        }"#;

        for (root, expected_id, expected_val) in [
            (QueryRoot::Human, "human-32", 10),
            (QueryRoot::Droid, "droid-99", 42),
        ] {
            let schema = schema(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((
                    graphql_value!({"character": {
                        "id": expected_id,
                        "keyFeature": {"value": expected_val},
                    }}),
                    vec![],
                )),
            );
        }
    }
}

mod nullable_argument_subtyping {
    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = [Human, Droid])]
    struct Character {
        id: prelude::Option<prelude::String>,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    struct Droid {
        id: prelude::String,
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        fn id(&self, is_present: prelude::Option<bool>) -> &str {
            if is_present.unwrap_or_default() {
                &self.id
            } else {
                "missing"
            }
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "human-32",
                    "homePlanet": "earth",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    droidId: id(isPresent: true)
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
                    "droidId": "droid-99",
                    "primaryFunction": "run",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_field() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        for (root, expected_id) in [
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "missing"),
        ] {
            let schema = schema(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod simple_subtyping {
    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = [ResourceValue, Endpoint])]
    struct Node {
        id: prelude::Option<ID>,
    }

    #[derive(GraphQLInterface)]
    #[graphql(impl = NodeValue, for = Endpoint)]
    struct Resource {
        id: ID,
        url: prelude::Option<prelude::String>,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [ResourceValue, NodeValue])]
    struct Endpoint {
        id: ID,
        url: prelude::String,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn node() -> NodeValue {
            Endpoint {
                id: ID::new("1"),
                url: "2".into(),
            }
            .into()
        }

        fn resource() -> ResourceValue {
            Endpoint {
                id: ID::new("3"),
                url: "4".into(),
            }
            .into()
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        for name in ["Node", "Resource"] {
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
                Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn resolves_node() {
        const DOC: &str = r#"{
            node {
                id
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"node": {"id": "1"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn resolves_node_on_resource() {
        const DOC: &str = r#"{
            node {
                ... on Resource {
                    id
                    url
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"node": {
                    "id": "1",
                    "url": "2",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_node_on_endpoint() {
        const DOC: &str = r#"{
            node {
                ... on Endpoint {
                    id
                    url
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"node": {
                    "id": "1",
                    "url": "2",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_resource() {
        const DOC: &str = r#"{
            resource {
                id
                url
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"resource": {
                    "id": "3",
                    "url": "4",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_resource_on_endpoint() {
        const DOC: &str = r#"{
            resource {
                ... on Endpoint {
                    id
                    url
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"resource": {
                    "id": "3",
                    "url": "4",
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn registers_possible_types() {
        for name in ["Node", "Resource"] {
            let doc = format!(
                r#"{{
                    __type(name: "{name}") {{
                        possibleTypes {{
                            kind
                            name
                        }}
                    }}
                }}"#,
            );

            let schema = schema(QueryRoot);

            assert_eq!(
                execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
                Ok((
                    graphql_value!({"__type": {"possibleTypes": [
                        {"kind": "OBJECT", "name": "Endpoint"},
                    ]}}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn registers_interfaces() {
        let schema = schema(QueryRoot);

        for (name, interfaces) in [
            ("Node", graphql_value!([])),
            (
                "Resource",
                graphql_value!([{"kind": "INTERFACE", "name": "Node"}]),
            ),
            (
                "Endpoint",
                graphql_value!([
                    {"kind": "INTERFACE", "name": "Node"},
                    {"kind": "INTERFACE", "name": "Resource"},
                ]),
            ),
        ] {
            let doc = format!(
                r#"{{
                   __type(name: "{name}") {{
                       interfaces {{
                           kind
                           name
                       }}
                   }}
                }}"#,
            );

            assert_eq!(
                execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
                Ok((
                    graphql_value!({"__type": {"interfaces": interfaces}}),
                    vec![],
                )),
            );
        }
    }
}

mod branching_subtyping {
    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = [HumanValue, DroidValue, Luke, R2D2])]
    struct Node {
        id: ID,
    }

    #[derive(GraphQLInterface)]
    #[graphql(for = [HumanConnection, DroidConnection])]
    struct Connection {
        nodes: Vec<NodeValue>,
    }

    #[derive(GraphQLInterface)]
    #[graphql(impl = NodeValue, for = Luke)]
    struct Human {
        id: ID,
        home_planet: prelude::String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = ConnectionValue)]
    struct HumanConnection {
        nodes: Vec<HumanValue>,
    }

    #[derive(GraphQLInterface)]
    #[graphql(impl = NodeValue, for = R2D2)]
    struct Droid {
        id: ID,
        primary_function: prelude::String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = ConnectionValue)]
    struct DroidConnection {
        nodes: Vec<DroidValue>,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [HumanValue, NodeValue])]
    struct Luke {
        id: ID,
        home_planet: prelude::String,
        father: prelude::String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = [DroidValue, NodeValue])]
    struct R2D2 {
        id: ID,
        primary_function: prelude::String,
        charge: f64,
    }

    enum QueryRoot {
        Luke,
        R2D2,
    }

    #[graphql_object]
    impl QueryRoot {
        fn crew(&self) -> ConnectionValue {
            match self {
                Self::Luke => HumanConnection {
                    nodes: vec![Luke {
                        id: ID::new("1"),
                        home_planet: "earth".into(),
                        father: "SPOILER".into(),
                    }
                    .into()],
                }
                .into(),
                Self::R2D2 => DroidConnection {
                    nodes: vec![R2D2 {
                        id: ID::new("2"),
                        primary_function: "roll".into(),
                        charge: 146.0,
                    }
                    .into()],
                }
                .into(),
            }
        }
    }

    #[tokio::test]
    async fn is_graphql_interface() {
        for name in ["Node", "Connection", "Human", "Droid"] {
            let doc = format!(
                r#"{{
                    __type(name: "{name}") {{
                        kind
                    }}
                }}"#,
            );

            let schema = schema(QueryRoot::Luke);

            assert_eq!(
                execute(&doc, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"__type": {"kind": "INTERFACE"}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn resolves_human_connection() {
        const DOC: &str = r#"{
            crew {
                ... on HumanConnection {
                    nodes {
                        id
                        homePlanet
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot::Luke);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"crew": {
                    "nodes": [{
                        "id": "1",
                        "homePlanet": "earth",
                    }],
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            crew {
                nodes {
                    ... on Human {
                        id
                        homePlanet
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot::Luke);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"crew": {
                    "nodes": [{
                        "id": "1",
                        "homePlanet": "earth",
                    }],
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_luke() {
        const DOC: &str = r#"{
            crew {
                nodes {
                    ... on Luke {
                        id
                        homePlanet
                        father
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot::Luke);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"crew": {
                    "nodes": [{
                        "id": "1",
                        "homePlanet": "earth",
                        "father": "SPOILER",
                    }],
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid_connection() {
        const DOC: &str = r#"{
            crew {
                ... on DroidConnection {
                    nodes {
                        id
                        primaryFunction
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot::R2D2);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"crew": {
                    "nodes": [{
                        "id": "2",
                        "primaryFunction": "roll",
                    }],
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            crew {
                nodes {
                    ... on Droid {
                        id
                        primaryFunction
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot::R2D2);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"crew": {
                    "nodes": [{
                        "id": "2",
                        "primaryFunction": "roll",
                    }],
                }}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_r2d2() {
        const DOC: &str = r#"{
            crew {
                nodes {
                    ... on R2D2 {
                        id
                        primaryFunction
                        charge
                    }
                }
            }
        }"#;

        let schema = schema(QueryRoot::R2D2);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"crew": {
                    "nodes": [{
                        "id": "2",
                        "primaryFunction": "roll",
                        "charge": 146.0,
                    }],
                }}),
                vec![],
            )),
        );
    }
}

mod preserves_visibility {
    use super::*;

    #[allow(dead_code)]
    type Foo = self::inner::CharacterValue;

    pub(crate) mod inner {
        use super::*;

        #[derive(GraphQLInterface)]
        #[graphql(for = Human)]
        pub(crate) struct Character {
            id: prelude::String,
        }

        #[derive(GraphQLObject)]
        #[graphql(impl = CharacterValue)]
        pub(crate) struct Human {
            id: prelude::String,
            home_planet: prelude::String,
        }
    }
}

mod has_no_missing_docs {
    #![deny(missing_docs)]

    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = Human)]
    pub struct Character {
        pub id: prelude::String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    pub struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }
}
