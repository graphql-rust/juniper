//! Tests for `#[derive(GraphQLInterface)]` macro.

use std::marker::PhantomData;

use juniper::{
    execute, graphql_object, graphql_value, graphql_vars, DefaultScalarValue, FieldError,
    FieldResult, GraphQLInterface, GraphQLObject, GraphQLUnion, IntoFieldError, ScalarValue,
};

use crate::util::{schema, schema_with_scalar};

mod no_implers {
    use super::*;

    #[derive(GraphQLInterface)]
    struct Character {
        id: String,
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
        id: String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
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
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
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

        for object in &["Human", "Droid"] {
            let doc = format!(
                r#"{{
                   __type(name: "{}") {{
                       interfaces {{
                           kind
                           name
                       }}
                   }}
                }}"#,
                object,
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
        id: String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterEnum)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
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
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
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
        id: String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
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
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
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

        for object in &["Human", "Droid"] {
            let doc = format!(
                r#"{{
                   __type(name: "{}") {{
                       interfaces {{
                           kind
                           name
                       }}
                   }}
                }}"#,
                object,
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
        id: Result<String, CustomError>,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        fn id(&self) -> Result<String, CustomError> {
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
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
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
        id: String,

        #[graphql(skip)]
        _phantom: PhantomData<(A, B)>,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
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
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
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
        id: String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
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
        id: String,

        #[deprecated]
        a: String,

        #[deprecated(note = "Use `id`.")]
        b: String,
    }

    struct Human {
        id: String,
        home_planet: String,
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

        fn b() -> String {
            "b".to_owned()
        }
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
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
        id: String,

        #[graphql(deprecated)]
        #[deprecated(note = "Should be omitted.")]
        a: String,

        b: String,
    }

    struct Human {
        id: String,
        home_planet: String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Human {
        fn my_id(&self, #[graphql(name = "myName")] _: Option<String>) -> &str {
            &self.id
        }

        fn home_planet(&self) -> &str {
            &self.home_planet
        }

        fn a() -> String {
            "a".to_owned()
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
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
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
        id: String,
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
        id: String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue, scalar = DefaultScalarValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
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
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}

mod custom_scalar {
    use crate::custom_scalar::MyScalarValue;

    use super::*;

    #[derive(GraphQLInterface)]
    #[graphql(for = [Human, Droid], scalar = MyScalarValue)]
    struct Character {
        id: String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue, scalar = MyScalarValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
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
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema_with_scalar::<MyScalarValue, _, _>(*root);

            let expected_id: &str = *expected_id;
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
        id: FieldResult<String, S>,
    }

    #[derive(GraphQLObject)]
    #[graphql(scalar = S: ScalarValue, impl = CharacterValue<S>)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
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
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
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
    #[graphql(for = [Human, Droid], scalar = S: ScalarValue + Clone)]
    struct Character {
        id: String,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue, scalar = S: ScalarValue + Clone)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue, scalar = S: ScalarValue + Clone)]
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
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
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
        id: String,

        #[graphql(ignore)]
        ignored: Option<Human>,

        #[graphql(skip)]
        skipped: i32,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
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
        id: Option<String>,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
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
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "droid-99"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
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
        id: Option<String>,
        key_feature: KeyFeature,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
        key_feature: Knowledge,
    }

    struct Droid {
        id: String,
        primary_function: String,
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
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                    key_feature: Knowledge { value: 10 },
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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

        for (root, expected_id, expected_val) in &[
            (QueryRoot::Human, "human-32", 10),
            (QueryRoot::Droid, "droid-99", 42),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            let expected_val = *expected_val;
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
        id: Option<String>,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    struct Human {
        id: String,
        home_planet: String,
    }

    struct Droid {
        id: String,
        primary_function: String,
    }

    #[graphql_object(impl = CharacterValue)]
    impl Droid {
        fn id(&self, is_present: Option<bool>) -> &str {
            is_present
                .unwrap_or_default()
                .then(|| self.id.as_str())
                .unwrap_or("missing")
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
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }
                .into(),
                Self::Droid => Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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

        for (root, expected_id) in &[
            (QueryRoot::Human, "human-32"),
            (QueryRoot::Droid, "missing"),
        ] {
            let schema = schema(*root);

            let expected_id: &str = *expected_id;
            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected_id}}), vec![])),
            );
        }
    }
}
