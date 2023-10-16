//! Tests for `#[graphql_union]` macro.

pub mod common;

use juniper::{
    execute, graphql_object, graphql_union, graphql_value, graphql_vars, DefaultScalarValue,
    GraphQLObject, ScalarValue,
};

use self::common::util::{schema, schema_with_scalar};

// Override `std::prelude` items to check whether macros expand hygienically.
#[allow(unused_imports)]
use self::common::hygiene::*;

#[derive(GraphQLObject)]
struct Human {
    id: prelude::String,
    home_planet: prelude::String,
}

#[derive(GraphQLObject)]
struct Droid {
    id: prelude::String,
    primary_function: prelude::String,
}

#[derive(GraphQLObject)]
struct Ewok {
    id: prelude::String,
    funny: bool,
}

pub enum CustomContext {
    Human,
    Droid,
    Ewok,
}
impl juniper::Context for CustomContext {}

#[derive(GraphQLObject)]
#[graphql(context = CustomContext)]
pub struct HumanCustomContext {
    id: prelude::String,
    home_planet: prelude::String,
}

#[derive(GraphQLObject)]
#[graphql(context = CustomContext)]
pub struct DroidCustomContext {
    id: prelude::String,
    primary_function: prelude::String,
}

#[derive(GraphQLObject)]
#[graphql(context = CustomContext)]
struct EwokCustomContext {
    id: prelude::String,
    funny: bool,
}

mod trivial {
    use super::*;

    #[graphql_union]
    trait Character {
        fn as_human(&self) -> prelude::Option<&Human> {
            None
        }
        fn as_droid(&self) -> prelude::Option<&Droid> {
            None
        }
    }

    impl Character for Human {
        fn as_human(&self) -> prelude::Option<&Human> {
            Some(self)
        }
    }

    impl Character for Droid {
        fn as_droid(&self) -> prelude::Option<&Droid> {
            Some(self)
        }
    }

    type DynCharacter<'a> = dyn Character + prelude::Send + prelude::Sync + 'a;

    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> prelude::Box<DynCharacter<'_>> {
            let ch: prelude::Box<DynCharacter<'_>> = match self {
                Self::Human => prelude::Box::new(Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }),
                Self::Droid => prelude::Box::new(Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }),
            };
            ch
        }
    }

    const DOC: &str = r#"{
        character {
            ... on Human {
                humanId: id
                homePlanet
            }
            ... on Droid {
                droidId: id
                primaryFunction
            }
        }
    }"#;

    #[tokio::test]
    async fn resolves_human() {
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
        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn is_graphql_union() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                kind
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"kind": "UNION"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_type_name() {
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

mod generic {
    use super::*;

    #[graphql_union]
    trait Character<A, B> {
        fn as_human(&self) -> prelude::Option<&Human> {
            None
        }
        fn as_droid(&self) -> prelude::Option<&Droid> {
            None
        }
    }

    impl<A, B> Character<A, B> for Human {
        fn as_human(&self) -> prelude::Option<&Human> {
            Some(self)
        }
    }

    impl<A, B> Character<A, B> for Droid {
        fn as_droid(&self) -> prelude::Option<&Droid> {
            Some(self)
        }
    }

    type DynCharacter<'a, A, B> = dyn Character<A, B> + prelude::Send + prelude::Sync + 'a;

    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> prelude::Box<DynCharacter<'_, u8, ()>> {
            let ch: prelude::Box<DynCharacter<'_, u8, ()>> = match self {
                Self::Human => prelude::Box::new(Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }),
                Self::Droid => prelude::Box::new(Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }),
            };
            ch
        }
    }

    const DOC: &str = r#"{
        character {
            ... on Human {
                humanId: id
                homePlanet
            }
            ... on Droid {
                droidId: id
                primaryFunction
            }
        }
    }"#;

    #[tokio::test]
    async fn resolves_human() {
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
        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_type_name_without_type_params() {
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
    #[graphql_union]
    trait Character {
        fn as_human(&self) -> prelude::Option<&Human> {
            None
        }
    }

    impl Character for Human {
        fn as_human(&self) -> prelude::Option<&Human> {
            Some(self)
        }
    }

    type DynCharacter<'a> = dyn Character + prelude::Send + prelude::Sync + 'a;

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> prelude::Box<DynCharacter<'_>> {
            prelude::Box::new(Human {
                id: "human-32".into(),
                home_planet: "earth".into(),
            })
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
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_doc_comment_as_description() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"description": "Rust docs."}}),
                vec![],
            )),
        );
    }
}

mod explicit_name_and_description {
    use super::*;

    /// Rust docs.
    #[graphql_union(name = "MyChar", desc = "My character.")]
    trait Character {
        fn as_human(&self) -> prelude::Option<&Human> {
            None
        }
    }

    impl Character for Human {
        fn as_human(&self) -> prelude::Option<&Human> {
            Some(self)
        }
    }

    type DynCharacter<'a> = dyn Character + prelude::Send + prelude::Sync + 'a;

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> prelude::Box<DynCharacter<'_>> {
            prelude::Box::new(Human {
                id: "human-32".into(),
                home_planet: "earth".into(),
            })
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
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_custom_name() {
        const DOC: &str = r#"{
            __type(name: "MyChar") {
                name
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((graphql_value!({"__type": {"name": "MyChar"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_custom_description() {
        const DOC: &str = r#"{
            __type(name: "MyChar") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"description": "My character."}}),
                vec![],
            )),
        );
    }
}

mod explicit_scalar {
    use super::*;

    #[graphql_union(scalar = DefaultScalarValue)]
    trait Character {
        fn as_human(&self) -> prelude::Option<&Human> {
            None
        }
        fn as_droid(&self) -> prelude::Option<&Droid> {
            None
        }
    }

    impl Character for Human {
        fn as_human(&self) -> prelude::Option<&Human> {
            Some(self)
        }
    }

    impl Character for Droid {
        fn as_droid(&self) -> prelude::Option<&Droid> {
            Some(self)
        }
    }

    type DynCharacter<'a> = dyn Character + prelude::Send + prelude::Sync + 'a;

    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn character(&self) -> prelude::Box<DynCharacter<'_>> {
            let ch: prelude::Box<DynCharacter<'_>> = match self {
                Self::Human => prelude::Box::new(Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }),
                Self::Droid => prelude::Box::new(Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }),
            };
            ch
        }
    }

    const DOC: &str = r#"{
        character {
            ... on Human {
                humanId: id
                homePlanet
            }
            ... on Droid {
                droidId: id
                primaryFunction
            }
        }
    }"#;

    #[tokio::test]
    async fn resolves_human() {
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
        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }
}

mod custom_scalar {
    use crate::common::MyScalarValue;

    use super::*;

    #[graphql_union(scalar = MyScalarValue)]
    trait Character {
        fn as_human(&self) -> prelude::Option<&Human> {
            None
        }
        fn as_droid(&self) -> prelude::Option<&Droid> {
            None
        }
    }

    impl Character for Human {
        fn as_human(&self) -> prelude::Option<&Human> {
            Some(self)
        }
    }

    impl Character for Droid {
        fn as_droid(&self) -> prelude::Option<&Droid> {
            Some(self)
        }
    }

    type DynCharacter<'a> = dyn Character + prelude::Send + prelude::Sync + 'a;

    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = MyScalarValue)]
    impl QueryRoot {
        fn character(&self) -> prelude::Box<DynCharacter<'_>> {
            let ch: prelude::Box<DynCharacter<'_>> = match self {
                Self::Human => prelude::Box::new(Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }),
                Self::Droid => prelude::Box::new(Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }),
            };
            ch
        }
    }

    const DOC: &str = r#"{
        character {
            ... on Human {
                humanId: id
                homePlanet
            }
            ... on Droid {
                droidId: id
                primaryFunction
            }
        }
    }"#;

    #[tokio::test]
    async fn resolves_human() {
        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot::Human);

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
        let schema = schema_with_scalar::<MyScalarValue, _, _>(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }
}

mod explicit_generic_scalar {
    use super::*;

    #[graphql_union(scalar = S)]
    trait Character<S: ScalarValue> {
        fn as_human(&self) -> prelude::Option<&Human> {
            None
        }
        fn as_droid(&self) -> prelude::Option<&Droid> {
            None
        }
    }

    impl<S: ScalarValue> Character<S> for Human {
        fn as_human(&self) -> prelude::Option<&Human> {
            Some(self)
        }
    }

    impl<S: ScalarValue> Character<S> for Droid {
        fn as_droid(&self) -> prelude::Option<&Droid> {
            Some(self)
        }
    }

    type DynCharacter<'a, S> = dyn Character<S> + prelude::Send + prelude::Sync + 'a;

    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character<__S: ScalarValue>(&self) -> prelude::Box<DynCharacter<'_, __S>> {
            let ch: prelude::Box<DynCharacter<'_, _>> = match self {
                Self::Human => prelude::Box::new(Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }),
                Self::Droid => prelude::Box::new(Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }),
            };
            ch
        }
    }

    const DOC: &str = r#"{
        character {
            ... on Human {
                humanId: id
                homePlanet
            }
            ... on Droid {
                droidId: id
                primaryFunction
            }
        }
    }"#;

    #[tokio::test]
    async fn resolves_human() {
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
        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }
}

mod bounded_generic_scalar {
    use super::*;

    #[graphql_union(scalar = S: ScalarValue + prelude::Clone)]
    trait Character {
        fn as_human(&self) -> prelude::Option<&Human> {
            None
        }
        fn as_droid(&self) -> prelude::Option<&Droid> {
            None
        }
    }

    impl Character for Human {
        fn as_human(&self) -> prelude::Option<&Human> {
            Some(self)
        }
    }

    impl Character for Droid {
        fn as_droid(&self) -> prelude::Option<&Droid> {
            Some(self)
        }
    }

    type DynCharacter<'a> = dyn Character + prelude::Send + prelude::Sync + 'a;

    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> prelude::Box<DynCharacter<'_>> {
            let ch: prelude::Box<DynCharacter<'_>> = match self {
                Self::Human => prelude::Box::new(Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }),
                Self::Droid => prelude::Box::new(Droid {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }),
            };
            ch
        }
    }

    const DOC: &str = r#"{
        character {
            ... on Human {
                humanId: id
                homePlanet
            }
            ... on Droid {
                droidId: id
                primaryFunction
            }
        }
    }"#;

    #[tokio::test]
    async fn resolves_human() {
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
        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }
}

mod explicit_custom_context {
    use super::*;

    #[graphql_union(context = CustomContext)]
    trait Character {
        fn as_human(&self) -> prelude::Option<&HumanCustomContext> {
            None
        }
        fn as_droid(&self) -> prelude::Option<&DroidCustomContext> {
            None
        }
    }

    impl Character for HumanCustomContext {
        fn as_human(&self) -> prelude::Option<&HumanCustomContext> {
            Some(self)
        }
    }

    impl Character for DroidCustomContext {
        fn as_droid(&self) -> prelude::Option<&DroidCustomContext> {
            Some(self)
        }
    }

    type DynCharacter<'a> = dyn Character + prelude::Send + prelude::Sync + 'a;

    struct QueryRoot;

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn character(&self, ctx: &CustomContext) -> prelude::Box<DynCharacter<'_>> {
            let ch: prelude::Box<DynCharacter<'_>> = match ctx {
                CustomContext::Human => prelude::Box::new(HumanCustomContext {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }),
                CustomContext::Droid => prelude::Box::new(DroidCustomContext {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }),
                _ => unimplemented!(),
            };
            ch
        }
    }

    const DOC: &str = r#"{
        character {
            ... on HumanCustomContext {
                humanId: id
                homePlanet
            }
            ... on DroidCustomContext {
                droidId: id
                primaryFunction
            }
        }
    }"#;

    #[tokio::test]
    async fn resolves_human() {
        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext::Human).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext::Droid).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }
}

mod inferred_custom_context {
    use super::*;

    #[graphql_union]
    trait Character {
        fn as_human(&self, _: &CustomContext) -> prelude::Option<&HumanCustomContext> {
            None
        }
        fn as_droid(&self, _: &()) -> prelude::Option<&DroidCustomContext> {
            None
        }
    }

    impl Character for HumanCustomContext {
        fn as_human(&self, _: &CustomContext) -> prelude::Option<&HumanCustomContext> {
            Some(self)
        }
    }

    impl Character for DroidCustomContext {
        fn as_droid(&self, _: &()) -> prelude::Option<&DroidCustomContext> {
            Some(self)
        }
    }

    type DynCharacter<'a> = dyn Character + prelude::Send + prelude::Sync + 'a;

    struct QueryRoot;

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn character(&self, ctx: &CustomContext) -> prelude::Box<DynCharacter<'_>> {
            let ch: prelude::Box<DynCharacter<'_>> = match ctx {
                CustomContext::Human => prelude::Box::new(HumanCustomContext {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }),
                CustomContext::Droid => prelude::Box::new(DroidCustomContext {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }),
                _ => unimplemented!(),
            };
            ch
        }
    }

    const DOC: &str = r#"{
        character {
            ... on HumanCustomContext {
                humanId: id
                homePlanet
            }
            ... on DroidCustomContext {
                droidId: id
                primaryFunction
            }
        }
    }"#;

    #[tokio::test]
    async fn resolves_human() {
        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext::Human).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext::Droid).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }
}

mod ignored_method {
    use super::*;

    #[graphql_union]
    trait Character {
        fn as_human(&self) -> prelude::Option<&Human> {
            None
        }
        #[graphql(ignore)]
        fn ignored(&self) -> prelude::Option<&Ewok> {
            None
        }
        #[graphql(skip)]
        fn skipped(&self) {}
    }

    impl Character for Human {
        fn as_human(&self) -> prelude::Option<&Human> {
            Some(self)
        }
    }

    type DynCharacter<'a> = dyn Character + prelude::Send + prelude::Sync + 'a;

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> prelude::Box<DynCharacter<'_>> {
            prelude::Box::new(Human {
                id: "human-32".into(),
                home_planet: "earth".into(),
            })
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
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn ignores_ewok() {
        const DOC: &str = r#"{
            __type(name: "Character") {
                possibleTypes {
                    name
                }
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &()).await,
            Ok((
                graphql_value!({"__type": {"possibleTypes": [{"name": "Human"}]}}),
                vec![],
            )),
        );
    }
}

mod external_resolver {
    use super::*;

    #[graphql_union(context = Database)]
    #[graphql_union(on Droid = DynCharacter::as_droid)]
    trait Character {
        fn as_human(&self) -> prelude::Option<&Human> {
            None
        }
    }

    impl Character for Human {
        fn as_human(&self) -> prelude::Option<&Human> {
            Some(self)
        }
    }

    impl Character for Droid {}

    type DynCharacter<'a> = dyn Character + prelude::Send + prelude::Sync + 'a;

    impl<'a> DynCharacter<'a> {
        fn as_droid<'db>(&self, db: &'db Database) -> prelude::Option<&'db Droid> {
            db.droid.as_ref()
        }
    }

    struct Database {
        droid: prelude::Option<Droid>,
    }
    impl juniper::Context for Database {}

    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(context = Database)]
    impl QueryRoot {
        fn character(&self) -> prelude::Box<DynCharacter<'_>> {
            let ch: prelude::Box<DynCharacter<'_>> = match self {
                Self::Human => prelude::Box::new(Human {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }),
                Self::Droid => prelude::Box::new(Droid {
                    id: "?????".into(),
                    primary_function: "???".into(),
                }),
            };
            ch
        }
    }

    const DOC: &str = r#"{
        character {
            ... on Human {
                humanId: id
                homePlanet
            }
            ... on Droid {
                droidId: id
                primaryFunction
            }
        }
    }"#;

    #[tokio::test]
    async fn resolves_human() {
        let schema = schema(QueryRoot::Human);
        let db = Database { droid: None };

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &db).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        let schema = schema(QueryRoot::Droid);
        let db = Database {
            droid: Some(Droid {
                id: "droid-99".into(),
                primary_function: "run".into(),
            }),
        };

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &db).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }
}

mod full_featured {
    use super::*;

    /// Rust doc.
    #[graphql_union(name = "MyChar")]
    #[graphql_union(description = "My character.")]
    #[graphql_union(context = CustomContext, scalar = DefaultScalarValue)]
    #[graphql_union(on EwokCustomContext = resolve_ewok)]
    trait Character<T> {
        fn as_human(&self, _: &()) -> prelude::Option<&HumanCustomContext> {
            None
        }
        fn as_droid(&self) -> prelude::Option<&DroidCustomContext> {
            None
        }
        #[graphql(ignore)]
        fn as_ewok(&self) -> prelude::Option<&EwokCustomContext> {
            None
        }
        #[graphql(ignore)]
        fn ignored(&self) {}
    }

    impl<T> Character<T> for HumanCustomContext {
        fn as_human(&self, _: &()) -> prelude::Option<&HumanCustomContext> {
            Some(self)
        }
    }

    impl<T> Character<T> for DroidCustomContext {
        fn as_droid(&self) -> prelude::Option<&DroidCustomContext> {
            Some(self)
        }
    }

    impl<T> Character<T> for EwokCustomContext {
        fn as_ewok(&self) -> prelude::Option<&EwokCustomContext> {
            Some(self)
        }
    }

    type DynCharacter<'a, T> = dyn Character<T> + prelude::Send + prelude::Sync + 'a;

    fn resolve_ewok<'a, T>(
        ewok: &'a DynCharacter<'a, T>,
        _: &CustomContext,
    ) -> prelude::Option<&'a EwokCustomContext> {
        ewok.as_ewok()
    }

    struct QueryRoot;

    #[graphql_object(context = CustomContext, scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn character(&self, ctx: &CustomContext) -> prelude::Box<DynCharacter<'_, ()>> {
            let ch: prelude::Box<DynCharacter<'_, ()>> = match ctx {
                CustomContext::Human => prelude::Box::new(HumanCustomContext {
                    id: "human-32".into(),
                    home_planet: "earth".into(),
                }),
                CustomContext::Droid => prelude::Box::new(DroidCustomContext {
                    id: "droid-99".into(),
                    primary_function: "run".into(),
                }),
                CustomContext::Ewok => prelude::Box::new(EwokCustomContext {
                    id: "ewok-1".into(),
                    funny: true,
                }),
            };
            ch
        }
    }

    const DOC: &str = r#"{
        character {
            ... on HumanCustomContext {
                humanId: id
                homePlanet
            }
            ... on DroidCustomContext {
                droidId: id
                primaryFunction
            }
            ... on EwokCustomContext {
                ewokId: id
                funny
            }
        }
    }"#;

    #[tokio::test]
    async fn resolves_human() {
        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext::Human).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext::Droid).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_ewok() {
        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext::Ewok).await,
            Ok((
                graphql_value!({"character": {"ewokId": "ewok-1", "funny": true}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn uses_custom_name() {
        const DOC: &str = r#"{
            __type(name: "MyChar") {
                name
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext::Ewok).await,
            Ok((graphql_value!({"__type": {"name": "MyChar"}}), vec![])),
        );
    }

    #[tokio::test]
    async fn uses_custom_description() {
        const DOC: &str = r#"{
            __type(name: "MyChar") {
                description
            }
        }"#;

        let schema = schema(QueryRoot);

        assert_eq!(
            execute(DOC, None, &schema, &graphql_vars! {}, &CustomContext::Ewok).await,
            Ok((
                graphql_value!({"__type": {"description": "My character."}}),
                vec![],
            )),
        );
    }
}
