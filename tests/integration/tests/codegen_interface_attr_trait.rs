//! Tests for `#[graphql_interface]` macro placed on a trait.

// Assert that `#[graphql_interface]` macro placed on a trait stops Clippy from enforcing `# Errors`
// and `# Panics` sections in GraphQL descriptions.
#![deny(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod common;

use juniper::{
    execute, graphql_interface, graphql_object, graphql_value, graphql_vars, DefaultScalarValue,
    Executor, FieldError, FieldResult, GraphQLInputObject, GraphQLObject, GraphQLUnion,
    IntoFieldError, ScalarValue, ID,
};

use self::common::util::{schema, schema_with_scalar};

// Override `std::prelude` items to check whether macros expand hygienically.
#[allow(unused_imports)]
use self::common::hygiene::*;

mod no_implers {
    use super::*;

    #[graphql_interface]
    trait Character {
        fn id(&self) -> &str;
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
    async fn uses_trait_name() {
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

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> &str;
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
    async fn uses_trait_name() {
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

    #[graphql_interface(enum = CharacterEnum, for = [Human, Droid])]
    trait Character {
        fn id(&self) -> &str;
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
    async fn uses_trait_name() {
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

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> &str;
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
    async fn uses_trait_name() {
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

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> prelude::Result<&str, CustomError>;
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

    #[graphql_interface(for = [Human, Droid])]
    trait Character<A = (), B: ?Sized = ()> {
        fn id(&self) -> &str;
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
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
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
    async fn uses_trait_name_without_type_params() {
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

mod argument {
    use super::*;

    #[graphql_interface(for = Human)]
    trait Character {
        fn id_wide(&self, is_number: bool) -> &str;

        fn id_wide2(&self, is_number: bool, r#async: prelude::Option<i32>) -> &str;
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

        fn home_planet(&self) -> &str {
            &self.home_planet
        }

        async fn id_wide(&self, is_number: bool) -> &str {
            if is_number {
                &self.id
            } else {
                "none"
            }
        }

        async fn id_wide2(&self, is_number: bool, _async: prelude::Option<i32>) -> &str {
            if is_number {
                &self.id
            } else {
                "none"
            }
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
        let schema = schema(QueryRoot);

        for (input, expected) in [
            (
                "{ character { idWide(isNumber: true), idWide2(isNumber: true) } }",
                "human-32",
            ),
            (
                "{ character { idWide(isNumber: false), idWide2(isNumber: false, async: 5) } }",
                "none",
            ),
        ] {
            assert_eq!(
                execute(input, None, &schema, &graphql_vars! {}, &()).await,
                Ok((
                    graphql_value!({"character": {
                        "idWide": expected,
                        "idWide2": expected,
                    }}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn camelcases_name() {
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
                graphql_value!({"__type": {"fields": [{
                    "name": "idWide",
                    "args": [
                        {"name": "isNumber"},
                    ],
                }, {
                    "name": "idWide2",
                    "args": [
                        {"name": "isNumber"},
                        {"name": "async"},
                    ],
                }]}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
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
            __type(name: "Character") {
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

    #[graphql_interface(for = Human)]
    trait Character {
        fn id(
            &self,
            #[graphql(default)] first: prelude::String,
            #[graphql(default = "second")] second: prelude::String,
            #[graphql(default = "t")] third: prelude::String,
        ) -> prelude::String;

        fn info(&self, #[graphql(default = Point { x: 1 })] coord: Point) -> i32;
    }

    struct Human;

    #[graphql_object(impl = CharacterValue)]
    impl Human {
        async fn info(&self, coord: Point) -> i32 {
            coord.x
        }

        async fn id(
            &self,
            first: prelude::String,
            second: prelude::String,
            third: prelude::String,
        ) -> prelude::String {
            format!("{first}|{second}&{third}")
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
    async fn resolves_id_field() {
        let schema = schema(QueryRoot);

        for (input, expected) in [
            ("{ character { id } }", "|second&t"),
            (r#"{ character { id(first: "first") } }"#, "first|second&t"),
            (r#"{ character { id(second: "") } }"#, "|&t"),
            (
                r#"{ character { id(first: "first", second: "") } }"#,
                "first|&t",
            ),
            (
                r#"{ character { id(first: "first", second: "", third: "") } }"#,
                "first|&",
            ),
        ] {
            assert_eq!(
                execute(input, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"id": expected}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn resolves_info_field() {
        let schema = schema(QueryRoot);

        for (input, expected) in [
            ("{ character { info } }", 1),
            ("{ character { info(coord: {x: 2}) } }", 2),
        ] {
            assert_eq!(
                execute(input, None, &schema, &graphql_vars! {}, &()).await,
                Ok((graphql_value!({"character": {"info": expected}}), vec![])),
            );
        }
    }

    #[tokio::test]
    async fn has_defaults() {
        const DOC: &str = r#"{
            __type(name: "Character") {
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
                        "name": "first",
                        "defaultValue": r#""""#,
                        "type": {"name": null, "ofType": {"name": "String"}},
                    }, {
                        "name": "second",
                        "defaultValue": r#""second""#,
                        "type": {"name": null, "ofType": {"name": "String"}},
                    }, {
                        "name": "third",
                        "defaultValue": r#""t""#,
                        "type": {"name": null, "ofType": {"name": "String"}},
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

    /// Rust docs.
    #[graphql_interface(for = Human)]
    trait Character {
        /// Rust `id` docs.
        /// Long.
        fn id(&self) -> &str;
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

    #[graphql_interface(for = Human)]
    trait Character {
        fn id(&self) -> &str;

        #[deprecated]
        fn a(&self) -> &str;

        #[deprecated(note = "Use `id`.")]
        fn b(&self) -> &str;
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
    #[graphql_interface(name = "MyChar", desc = "My character.", for = Human)]
    trait Character {
        /// Rust `id` docs.
        #[graphql(name = "myId", desc = "My character ID.", deprecated = "Not used.")]
        #[deprecated(note = "Should be omitted.")]
        fn id(
            &self,
            #[graphql(name = "myName", desc = "My argument.")] n: prelude::Option<prelude::String>,
        ) -> &str;

        #[graphql(deprecated)]
        #[deprecated(note = "Should be omitted.")]
        fn a(&self) -> &str;

        fn b(&self) -> &str;
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

    #[graphql_interface(rename_all = "none", for = Human)]
    trait Character {
        fn id(&self) -> &str;

        fn home_planet(&self, planet_name: prelude::String) -> prelude::String;

        fn r#async_info(&self, r#my_num: i32) -> i32;
    }

    struct Human;

    #[graphql_object(rename_all = "none", impl = CharacterValue)]
    impl Human {
        fn id() -> &'static str {
            "human-32"
        }

        async fn home_planet(planet_name: prelude::String) -> prelude::String {
            planet_name
        }

        async fn r#async_info(r#my_num: i32) -> i32 {
            r#my_num
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
                home_planet(planet_name: "earth")
                async_info(my_num: 3)
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {
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

    #[graphql_interface(for = [Human, Droid])]
    #[graphql_interface(scalar = DefaultScalarValue)]
    trait Character {
        fn id(&self) -> &str;
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

    #[graphql_interface(for = [Human, Droid], scalar = MyScalarValue)]
    trait Character {
        fn id(&self) -> &str;
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

    #[graphql_interface(for = [Human, Droid], scalar = S)]
    trait Character<S: ScalarValue = DefaultScalarValue> {
        fn id(&self) -> FieldResult<&str, S>;
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

    #[graphql_interface(for = [Human, Droid], scalar = S: ScalarValue + prelude::Clone)]
    trait Character {
        fn id(&self) -> &str;
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

mod explicit_custom_context {
    use super::*;

    struct CustomContext;

    impl juniper::Context for CustomContext {}

    #[graphql_interface(for = [Human, Droid], context = CustomContext)]
    trait Character {
        fn id<'a>(&'a self, context: &CustomContext) -> &'a str;

        fn info<'b>(&'b self, ctx: &()) -> &'b str;

        fn more<'c>(&'c self, #[graphql(context)] custom: &CustomContext) -> &'c str;
    }

    struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }

    #[graphql_object(impl = CharacterValue, context = CustomContext)]
    impl Human {
        async fn id<'a>(&'a self, _context: &CustomContext) -> &'a str {
            &self.id
        }

        async fn home_planet(&self) -> &str {
            &self.home_planet
        }

        fn info<'b>(&'b self, _ctx: &()) -> &'b str {
            &self.home_planet
        }

        fn more(&self, #[graphql(context)] _: &CustomContext) -> &'static str {
            "human"
        }
    }

    struct Droid {
        id: prelude::String,
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterValue, context = CustomContext)]
    impl Droid {
        #[allow(clippy::needless_lifetimes)] // intentionally
        async fn id<'a>(&'a self) -> &'a str {
            &self.id
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }

        #[allow(clippy::needless_lifetimes)] // intentionally
        async fn info<'b>(&'b self) -> &'b str {
            &self.primary_function
        }

        fn more(&self) -> &'static str {
            "droid"
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(context = CustomContext)]
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
            execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext).await,
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
            execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext).await,
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
    async fn resolves_fields() {
        const DOC: &str = r#"{
           character {
                id
                info
                more
            }
        }"#;

        for (root, expected_id, expected_info, expexted_more) in [
            (QueryRoot::Human, "human-32", "earth", "human"),
            (QueryRoot::Droid, "droid-99", "run", "droid"),
        ] {
            let schema = schema(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext).await,
                Ok((
                    graphql_value!({"character": {
                        "id": expected_id,
                        "info": expected_info,
                        "more": expexted_more,
                    }}),
                    vec![],
                )),
            );
        }
    }
}

mod inferred_custom_context_from_field {
    use super::*;

    struct CustomContext(prelude::String);

    impl juniper::Context for CustomContext {}

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id<'a>(&self, context: &'a CustomContext) -> &'a str;

        fn info(&self, context: &()) -> &str;
    }

    struct Human {
        home_planet: prelude::String,
    }

    #[graphql_object(impl = CharacterValue, context = CustomContext)]
    impl Human {
        fn id<'a>(&self, ctx: &'a CustomContext) -> &'a str {
            &ctx.0
        }

        fn home_planet(&self) -> &str {
            &self.home_planet
        }

        fn info<'b>(&'b self, _context: &()) -> &'b str {
            &self.home_planet
        }
    }

    struct Droid {
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterValue, context = CustomContext)]
    impl Droid {
        fn id<'a>(&self, ctx: &'a CustomContext) -> &'a str {
            &ctx.0
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }

        fn info(&self) -> &str {
            &self.primary_function
        }
    }

    #[derive(Clone, Copy)]
    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn character(&self) -> CharacterValue {
            match self {
                Self::Human => Human {
                    home_planet: "earth".into(),
                }
                .into(),
                Self::Droid => Droid {
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
        let ctx = CustomContext("in-ctx".into());

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &ctx).await,
            Ok((
                graphql_value!({"character": {
                    "humanId": "in-ctx",
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
        let ctx = CustomContext("in-droid".into());

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &ctx).await,
            Ok((
                graphql_value!({"character": {
                    "droidId": "in-droid",
                    "primaryFunction": "run",
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
                info
            }
        }"#;

        for (root, expected_id, expected_info) in [
            (QueryRoot::Human, "human-ctx", "earth"),
            (QueryRoot::Droid, "droid-ctx", "run"),
        ] {
            let schema = schema(root);
            let ctx = CustomContext(expected_id.into());

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &ctx).await,
                Ok((
                    graphql_value!({"character": {
                        "id": expected_id,
                        "info": expected_info,
                    }}),
                    vec![],
                )),
            );
        }
    }
}

mod executor {
    use super::*;

    #[graphql_interface(for = [Human, Droid], scalar = S)]
    trait Character<S: ScalarValue> {
        fn id<'a>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str;

        fn info<'b>(
            &'b self,
            arg: prelude::Option<i32>,
            #[graphql(executor)] another: &Executor<'_, '_, (), S>,
        ) -> &'b str;
    }

    struct Human {
        home_planet: prelude::String,
    }

    #[graphql_object(scalar = S: ScalarValue, impl = CharacterValue<S>)]
    impl Human {
        async fn id<'a, S: ScalarValue>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str {
            executor.look_ahead().field_name()
        }

        fn home_planet(&self) -> &str {
            &self.home_planet
        }

        #[allow(clippy::needless_lifetimes)] // intentionally
        async fn info<'b>(&'b self, _arg: prelude::Option<i32>) -> &'b str {
            &self.home_planet
        }
    }

    struct Droid {
        primary_function: prelude::String,
    }

    #[graphql_object(impl = CharacterValue<__S>)]
    impl Droid {
        fn id<'a, S: ScalarValue>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str {
            executor.look_ahead().field_name()
        }

        fn primary_function(&self) -> &str {
            &self.primary_function
        }

        async fn info<'b, S: ScalarValue>(
            &'b self,
            _arg: prelude::Option<i32>,
            _executor: &Executor<'_, '_, (), S>,
        ) -> &'b str {
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
        fn character(&self) -> CharacterValue<DefaultScalarValue> {
            match self {
                Self::Human => Human {
                    home_planet: "earth".into(),
                }
                .into(),
                Self::Droid => Droid {
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
                    "humanId": "humanId",
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
                    "droidId": "droidId",
                    "primaryFunction": "run",
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
                info
            }
        }"#;

        for (root, expected_info) in [(QueryRoot::Human, "earth"), (QueryRoot::Droid, "run")] {
            let schema = schema(root);

            assert_eq!(
                execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
                Ok((
                    graphql_value!({"character": {
                        "id": "id",
                        "info": expected_info,
                    }}),
                    vec![],
                )),
            );
        }
    }

    #[tokio::test]
    async fn not_arg() {
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

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"fields": [
                    {"name": "id", "args": []},
                    {"name": "info", "args": [{"name": "arg"}]},
                ]}}),
                vec![],
            )),
        );
    }
}

mod ignored_method {
    use super::*;

    #[graphql_interface(for = Human)]
    trait Character {
        fn id(&self) -> &str;

        #[graphql(ignore)]
        fn ignored(&self) -> prelude::Option<&Human> {
            None
        }

        #[graphql(skip)]
        fn skipped(&self) {}
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

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> prelude::Option<prelude::String>;
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

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> prelude::Option<prelude::String>;

        fn key_feature(&self) -> KeyFeature;
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

    #[graphql_interface(for = [Human, Droid])]
    trait Character {
        fn id(&self) -> prelude::Option<prelude::String>;
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

    #[graphql_interface(for = [ResourceValue, Endpoint])]
    trait Node {
        fn id() -> prelude::Option<ID>;
    }

    #[graphql_interface(impl = NodeValue, for = Endpoint)]
    trait Resource {
        fn id(&self) -> &ID;
        fn url(&self) -> prelude::Option<&str>;
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

    #[graphql_interface(for = [HumanValue, DroidValue, Luke, R2D2])]
    trait Node {
        fn id() -> ID;
    }

    #[graphql_interface(for = [HumanConnection, DroidConnection])]
    trait Connection {
        fn nodes(&self) -> &[NodeValue];
    }

    #[graphql_interface(impl = NodeValue, for = Luke)]
    trait Human {
        fn id(&self) -> &ID;
        fn home_planet(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = ConnectionValue)]
    struct HumanConnection {
        nodes: Vec<HumanValue>,
    }

    #[graphql_interface(impl = NodeValue, for = R2D2)]
    trait Droid {
        fn id() -> ID;
        fn primary_function() -> prelude::String;
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

        #[graphql_interface(for = Human)]
        pub(crate) trait Character {
            fn id(&self) -> &str;
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

    #[graphql_interface(for = Human)]
    pub trait Character {
        fn id(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = CharacterValue)]
    pub struct Human {
        id: prelude::String,
        home_planet: prelude::String,
    }
}
