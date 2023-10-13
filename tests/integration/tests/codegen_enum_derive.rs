//! Tests for `#[derive(GraphQLEnum)]` macro.

pub mod common;

use juniper::{
    execute, graphql_object, graphql_value, graphql_vars, parser::SourcePosition,
    DefaultScalarValue, ExecutionError, FieldError, GraphQLEnum, ScalarValue,
};

use self::common::util::{schema, schema_with_scalar};

// Override `std::prelude` items to check whether macros expand hygienically.
#[allow(unused_imports)]
use self::common::hygiene::*;

mod trivial {
    use super::*;

    #[derive(GraphQLEnum)]
    enum Character {
        Human,
        Droid,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn pass_as_is(character: Character) -> Character {
            character
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            passAsIs(character: HUMAN)
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"passAsIs": "HUMAN"}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_graphql_enum() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "ENUM"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_type_name() {
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

    #[tokio::test]
    async fn has_enum_values() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                enumValues {
                    name
                    description
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"enumValues": [
                    {"name": "HUMAN", "description": null},
                    {"name": "DROID", "description": null},
                ]}}),
                vec![],
            )),
        );
    }
}

mod ignored_variant {
    use super::*;

    #[derive(GraphQLEnum)]
    enum Character {
        Human,
        #[allow(dead_code)]
        #[graphql(ignore)]
        Droid,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn pass_as_is(character: Character) -> Character {
            character
        }

        fn droid() -> Character {
            Character::Droid
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            passAsIs(character: HUMAN)
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"passAsIs": "HUMAN"}), vec![])),
        );
    }

    #[tokio::test]
    async fn err_on_droid() {
        const DOC: &str = r#"{
            droid
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!(null),
                vec![ExecutionError::new(
                    SourcePosition::new(14, 1, 12),
                    &["droid"],
                    FieldError::from("Cannot resolve ignored enum variant"),
                )],
            )),
        );
    }

    #[tokio::test]
    async fn has_enum_values() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                enumValues {
                    name
                    description
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"enumValues": [
                    {"name": "HUMAN", "description": null},
                ]}}),
                vec![],
            )),
        );
    }
}

mod ignored_generic_variant {
    use super::*;

    #[derive(GraphQLEnum)]
    enum Character<T> {
        Human,
        Droid,
        #[allow(dead_code)]
        #[graphql(ignore)]
        Ignored(T),
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn pass_as_is(character: Character<()>) -> Character<()> {
            character
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            passAsIs(character: HUMAN)
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"passAsIs": "HUMAN"}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_enum_values() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                enumValues {
                    name
                    description
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"enumValues": [
                    {"name": "HUMAN", "description": null},
                    {"name": "DROID", "description": null},
                ]}}),
                vec![],
            )),
        );
    }
}

mod description_from_doc_comment {
    use super::*;

    /// Character doc.
    #[derive(GraphQLEnum)]
    enum Character {
        /// Human doc.
        Human,

        /// Droid doc.
        Droid,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn pass_as_is(character: Character) -> Character {
            character
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            passAsIs(character: HUMAN)
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"passAsIs": "HUMAN"}), vec![])),
        );
    }

    #[tokio::test]
    async fn is_graphql_enum() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "ENUM"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_type_name() {
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
    async fn has_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"description": "Character doc."}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_enum_values() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                enumValues {
                    name
                    description
                    isDeprecated
                    deprecationReason
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"enumValues": [
                    {
                        "name": "HUMAN",
                        "description": "Human doc.",
                        "isDeprecated": false,
                        "deprecationReason": null,
                    },
                    {
                        "name": "DROID",
                        "description": "Droid doc.",
                        "isDeprecated": false,
                        "deprecationReason": null,
                    },
                ]}}),
                vec![],
            )),
        );
    }
}

mod deprecation_from_attr {
    #![allow(deprecated)]

    use super::*;

    /// Character doc.
    #[derive(GraphQLEnum)]
    enum Character {
        /// Human doc.
        #[deprecated]
        Human,

        /// Droid doc.
        #[deprecated(note = "Reason")]
        Droid,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn pass_as_is(character: Character) -> Character {
            character
        }
    }

    #[tokio::test]
    async fn has_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"description": "Character doc."}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_enum_values() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                enumValues {
                    name
                    description
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"enumValues": []}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_enum_values_with_deprecated() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                enumValues(includeDeprecated: true) {
                    name
                    description
                    isDeprecated
                    deprecationReason
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"enumValues": [
                    {
                        "name": "HUMAN",
                        "description": "Human doc.",
                        "isDeprecated": true,
                        "deprecationReason": null,
                    },
                    {
                        "name": "DROID",
                        "description": "Droid doc.",
                        "isDeprecated": true,
                        "deprecationReason": "Reason",
                    },
                ]}}),
                vec![],
            )),
        );
    }
}

mod deprecation_from_graphql_attr {
    #![allow(deprecated)]

    use super::*;

    /// Character doc.
    #[derive(GraphQLEnum)]
    enum Character {
        /// Human doc.
        #[graphql(deprecated)]
        Human,

        /// Droid doc.
        #[graphql(deprecated = "Reason")]
        Droid,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn pass_as_is(character: Character) -> Character {
            character
        }
    }

    #[tokio::test]
    async fn has_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"description": "Character doc."}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_enum_values() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                enumValues {
                    name
                    description
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"enumValues": []}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_enum_values_with_deprecated() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                enumValues(includeDeprecated: true) {
                    name
                    description
                    isDeprecated
                    deprecationReason
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"enumValues": [
                    {
                        "name": "HUMAN",
                        "description": "Human doc.",
                        "isDeprecated": true,
                        "deprecationReason": null,
                    },
                    {
                        "name": "DROID",
                        "description": "Droid doc.",
                        "isDeprecated": true,
                        "deprecationReason": "Reason",
                    },
                ]}}),
                vec![],
            )),
        );
    }
}

mod explicit_name_description_and_deprecation {
    #![allow(deprecated)]

    use super::*;

    /// Doc comment.
    #[derive(GraphQLEnum)]
    #[graphql(name = "MyCharacter", desc = "Character doc.")]
    enum Character {
        /// Human doc.
        #[graphql(name = "MY_HUMAN", desc = "My human doc.", deprecated = "Not used.")]
        #[deprecated(note = "Should be omitted.")]
        Human,

        /// Droid doc.
        #[graphql(deprecated = "Reason")]
        Droid,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn pass_as_is(character: Character) -> Character {
            character
        }
    }

    #[tokio::test]
    async fn has_no_description() {
        const DOC: &str = r#"{
            __type(name: "MyCharacter") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"description": "Character doc."}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn has_enum_values() {
        const DOC: &str = r#"{
            __type(name: "MyCharacter") {
                enumValues {
                    name
                    description
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"enumValues": []}}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_enum_values_with_deprecated() {
        const DOC: &str = r#"{
            __type(name: "MyCharacter") {
                enumValues(includeDeprecated: true) {
                    name
                    description
                    isDeprecated
                    deprecationReason
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"enumValues": [
                    {
                        "name": "MY_HUMAN",
                        "description": "My human doc.",
                        "isDeprecated": true,
                        "deprecationReason": "Not used.",
                    },
                    {
                        "name": "DROID",
                        "description": "Droid doc.",
                        "isDeprecated": true,
                        "deprecationReason": "Reason",
                    },
                ]}}),
                vec![],
            )),
        );
    }
}

mod renamed_all_fields {
    use super::*;

    #[derive(GraphQLEnum)]
    #[graphql(rename_all = "none")]
    enum Character {
        Human,
        Droid,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn pass_as_is(character: Character) -> Character {
            character
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            passAsIs(character: Human)
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"passAsIs": "Human"}), vec![])),
        );
    }

    #[tokio::test]
    async fn has_enum_values() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                enumValues {
                    name
                    description
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"enumValues": [
                    {"name": "Human", "description": null},
                    {"name": "Droid", "description": null},
                ]}}),
                vec![],
            )),
        );
    }
}

mod explicit_scalar {
    use super::*;

    #[derive(GraphQLEnum)]
    #[graphql(scalar = DefaultScalarValue)]
    enum Character {
        Human,
        Droid,
    }

    struct QueryRoot;

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn pass_as_is(character: Character) -> Character {
            character
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            passAsIs(character: HUMAN)
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"passAsIs": "HUMAN"}), vec![])),
        );
    }
}

mod custom_scalar {
    use crate::common::MyScalarValue;

    use super::*;

    #[derive(GraphQLEnum)]
    #[graphql(scalar = MyScalarValue)]
    enum Character {
        Human,
        Droid,
    }

    struct QueryRoot;

    #[graphql_object(scalar = MyScalarValue)]
    impl QueryRoot {
        fn pass_as_is(character: Character) -> Character {
            character
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            passAsIs(character: HUMAN)
        }"#;

        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"passAsIs": "HUMAN"}), vec![])),
        );
    }
}

mod explicit_generic_scalar {
    use super::*;

    #[derive(GraphQLEnum)]
    #[graphql(scalar = S)]
    enum Character<S: ScalarValue> {
        Human,
        Droid,
        #[allow(dead_code)]
        #[graphql(ignore)]
        Scalar(S),
    }

    struct QueryRoot;

    #[graphql_object(scalar = S: ScalarValue)]
    impl QueryRoot {
        fn pass_as_is<S: ScalarValue>(character: Character<S>) -> Character<S> {
            character
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            passAsIs(character: HUMAN)
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"passAsIs": "HUMAN"}), vec![])),
        );
    }
}

mod bounded_generic_scalar {
    use super::*;

    #[derive(GraphQLEnum)]
    #[graphql(scalar = S: ScalarValue + prelude::Clone)]
    enum Character {
        Human,
        Droid,
    }

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn pass_as_is(character: Character) -> Character {
            character
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            passAsIs(character: HUMAN)
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"passAsIs": "HUMAN"}), vec![])),
        );
    }
}

mod explicit_custom_context {
    use super::*;

    struct CustomContext(prelude::String);

    impl juniper::Context for CustomContext {}

    #[derive(GraphQLEnum)]
    #[graphql(context = CustomContext)]
    enum Character {
        Human,
        Droid,
    }

    struct QueryRoot;

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn pass_as_is(character: Character, _ctx: &CustomContext) -> Character {
            character
        }
    }

    #[tokio::test]
    async fn resolves() {
        const DOC: &str = r#"{
            passAsIs(character: HUMAN)
        }"#;

        let schema = schema(QueryRoot);
        let ctx = CustomContext("ctx".into());

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &ctx).await,
            Ok((graphql_value!({"passAsIs": "HUMAN"}), vec![])),
        );
    }
}
