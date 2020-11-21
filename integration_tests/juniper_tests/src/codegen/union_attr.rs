//! Tests for `#[graphql_union]` macro.

use juniper::{
    execute, graphql_object, graphql_union, graphql_value, DefaultScalarValue, EmptyMutation,
    EmptySubscription, GraphQLObject, GraphQLType, RootNode, ScalarValue, Variables,
};

#[derive(GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

#[derive(GraphQLObject)]
struct Ewok {
    id: String,
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
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
#[graphql(context = CustomContext)]
pub struct DroidCustomContext {
    id: String,
    primary_function: String,
}

#[derive(GraphQLObject)]
#[graphql(context = CustomContext)]
struct EwokCustomContext {
    id: String,
    funny: bool,
}

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

    #[graphql_union]
    trait Character {
        fn as_human(&self) -> Option<&Human> {
            None
        }
        fn as_droid(&self) -> Option<&Droid> {
            None
        }
    }

    impl Character for Human {
        fn as_human(&self) -> Option<&Human> {
            Some(&self)
        }
    }

    impl Character for Droid {
        fn as_droid(&self) -> Option<&Droid> {
            Some(&self)
        }
    }

    type DynCharacter<'a> = dyn Character + Send + Sync + 'a;

    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"description": None}}), vec![])),
        );
    }
}

mod generic {
    use super::*;

    #[graphql_union]
    trait Character<A, B> {
        fn as_human(&self) -> Option<&Human> {
            None
        }
        fn as_droid(&self) -> Option<&Droid> {
            None
        }
    }

    impl<A, B> Character<A, B> for Human {
        fn as_human(&self) -> Option<&Human> {
            Some(&self)
        }
    }

    impl<A, B> Character<A, B> for Droid {
        fn as_droid(&self) -> Option<&Droid> {
            Some(&self)
        }
    }

    type DynCharacter<'a, A, B> = dyn Character<A, B> + Send + Sync + 'a;

    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_, u8, ()>> {
            let ch: Box<DynCharacter<'_, u8, ()>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((graphql_value!({"__type": {"name": "Character"}}), vec![])),
        );
    }
}

mod description_from_doc_comment {
    use super::*;

    /// Rust docs.
    #[graphql_union]
    trait Character {
        fn as_human(&self) -> Option<&Human> {
            None
        }
    }

    impl Character for Human {
        fn as_human(&self) -> Option<&Human> {
            Some(&self)
        }
    }

    type DynCharacter<'a> = dyn Character + Send + Sync + 'a;

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            Box::new(Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
        fn as_human(&self) -> Option<&Human> {
            None
        }
    }

    impl Character for Human {
        fn as_human(&self) -> Option<&Human> {
            Some(&self)
        }
    }

    type DynCharacter<'a> = dyn Character + Send + Sync + 'a;

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            Box::new(Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
        fn as_human(&self) -> Option<&Human> {
            None
        }
        fn as_droid(&self) -> Option<&Droid> {
            None
        }
    }

    impl Character for Human {
        fn as_human(&self) -> Option<&Human> {
            Some(&self)
        }
    }

    impl Character for Droid {
        fn as_droid(&self) -> Option<&Droid> {
            Some(&self)
        }
    }

    type DynCharacter<'a> = dyn Character + Send + Sync + 'a;

    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }
}

mod custom_scalar {
    use crate::custom_scalar::MyScalarValue;

    use super::*;

    #[graphql_union(scalar = MyScalarValue)]
    trait Character {
        fn as_human(&self) -> Option<&Human> {
            None
        }
        fn as_droid(&self) -> Option<&Droid> {
            None
        }
    }

    impl Character for Human {
        fn as_human(&self) -> Option<&Human> {
            Some(&self)
        }
    }

    impl Character for Droid {
        fn as_droid(&self) -> Option<&Droid> {
            Some(&self)
        }
    }

    type DynCharacter<'a> = dyn Character + Send + Sync + 'a;

    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(scalar = MyScalarValue)]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
        fn as_human(&self) -> Option<&HumanCustomContext> {
            None
        }
        fn as_droid(&self) -> Option<&DroidCustomContext> {
            None
        }
    }

    impl Character for HumanCustomContext {
        fn as_human(&self) -> Option<&HumanCustomContext> {
            Some(&self)
        }
    }

    impl Character for DroidCustomContext {
        fn as_droid(&self) -> Option<&DroidCustomContext> {
            Some(&self)
        }
    }

    type DynCharacter<'a> = dyn Character + Send + Sync + 'a;

    struct QueryRoot;

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn character(&self, ctx: &CustomContext) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match ctx {
                CustomContext::Human => Box::new(HumanCustomContext {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                CustomContext::Droid => Box::new(DroidCustomContext {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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
            execute(DOC, None, &schema, &Variables::new(), &CustomContext::Human).await,
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
            execute(DOC, None, &schema, &Variables::new(), &CustomContext::Droid).await,
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
        fn as_human(&self, _: &CustomContext) -> Option<&HumanCustomContext> {
            None
        }
        fn as_droid(&self, _: &()) -> Option<&DroidCustomContext> {
            None
        }
    }

    impl Character for HumanCustomContext {
        fn as_human(&self, _: &CustomContext) -> Option<&HumanCustomContext> {
            Some(&self)
        }
    }

    impl Character for DroidCustomContext {
        fn as_droid(&self, _: &()) -> Option<&DroidCustomContext> {
            Some(&self)
        }
    }

    type DynCharacter<'a> = dyn Character + Send + Sync + 'a;

    struct QueryRoot;

    #[graphql_object(context = CustomContext)]
    impl QueryRoot {
        fn character(&self, ctx: &CustomContext) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match ctx {
                CustomContext::Human => Box::new(HumanCustomContext {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                CustomContext::Droid => Box::new(DroidCustomContext {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
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
            execute(DOC, None, &schema, &Variables::new(), &CustomContext::Human).await,
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
            execute(DOC, None, &schema, &Variables::new(), &CustomContext::Droid).await,
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
        fn as_human(&self) -> Option<&Human> {
            None
        }
        #[graphql(ignore)]
        fn ignored(&self) -> Option<&Ewok> {
            None
        }
        #[graphql(skip)]
        fn skipped(&self) {}
    }

    impl Character for Human {
        fn as_human(&self) -> Option<&Human> {
            Some(&self)
        }
    }

    type DynCharacter<'a> = dyn Character + Send + Sync + 'a;

    struct QueryRoot;

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            Box::new(Human {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
        fn as_human(&self) -> Option<&Human> {
            None
        }
    }

    impl Character for Human {
        fn as_human(&self) -> Option<&Human> {
            Some(&self)
        }
    }

    impl Character for Droid {}

    type DynCharacter<'a> = dyn Character + Send + Sync + 'a;

    impl<'a> DynCharacter<'a> {
        fn as_droid<'db>(&self, db: &'db Database) -> Option<&'db Droid> {
            db.droid.as_ref()
        }
    }

    struct Database {
        droid: Option<Droid>,
    }
    impl juniper::Context for Database {}

    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object(context = Database)]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "?????".to_string(),
                    primary_function: "???".to_string(),
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
            execute(DOC, None, &schema, &Variables::new(), &db).await,
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
                id: "droid-99".to_string(),
                primary_function: "run".to_string(),
            }),
        };

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &db).await,
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
        fn as_human(&self, _: &()) -> Option<&HumanCustomContext> {
            None
        }
        fn as_droid(&self) -> Option<&DroidCustomContext> {
            None
        }
        #[graphql(ignore)]
        fn as_ewok(&self) -> Option<&EwokCustomContext> {
            None
        }
        #[graphql(ignore)]
        fn ignored(&self) {}
    }

    impl<T> Character<T> for HumanCustomContext {
        fn as_human(&self, _: &()) -> Option<&HumanCustomContext> {
            Some(&self)
        }
    }

    impl<T> Character<T> for DroidCustomContext {
        fn as_droid(&self) -> Option<&DroidCustomContext> {
            Some(&self)
        }
    }

    impl<T> Character<T> for EwokCustomContext {
        fn as_ewok(&self) -> Option<&EwokCustomContext> {
            Some(&self)
        }
    }

    type DynCharacter<'a, T> = dyn Character<T> + Send + Sync + 'a;

    fn resolve_ewok<'a, T>(
        ewok: &'a DynCharacter<'a, T>,
        _: &CustomContext,
    ) -> Option<&'a EwokCustomContext> {
        ewok.as_ewok()
    }

    struct QueryRoot;

    #[graphql_object(context = CustomContext, scalar = DefaultScalarValue)]
    impl QueryRoot {
        fn character(&self, ctx: &CustomContext) -> Box<DynCharacter<'_, ()>> {
            let ch: Box<DynCharacter<'_, ()>> = match ctx {
                CustomContext::Human => Box::new(HumanCustomContext {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                CustomContext::Droid => Box::new(DroidCustomContext {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
                CustomContext::Ewok => Box::new(EwokCustomContext {
                    id: "ewok-1".to_string(),
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
            execute(DOC, None, &schema, &Variables::new(), &CustomContext::Human).await,
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
            execute(DOC, None, &schema, &Variables::new(), &CustomContext::Droid).await,
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
            execute(DOC, None, &schema, &Variables::new(), &CustomContext::Ewok).await,
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
            execute(DOC, None, &schema, &Variables::new(), &CustomContext::Ewok).await,
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
            execute(DOC, None, &schema, &Variables::new(), &CustomContext::Ewok).await,
            Ok((
                graphql_value!({"__type": {"description": "My character."}}),
                vec![],
            )),
        );
    }
}
