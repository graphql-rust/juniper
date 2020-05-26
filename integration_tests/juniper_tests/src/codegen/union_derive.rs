// Test for union's derive macro

use derive_more::From;
#[cfg(test)]
use fnv::FnvHashMap;
use juniper::GraphQLUnion;

#[cfg(test)]
use juniper::{
    self, execute, DefaultScalarValue, EmptyMutation, EmptySubscription, GraphQLType, RootNode,
    Value, Variables,
};

#[derive(juniper::GraphQLObject)]
pub struct Human {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
pub struct Droid {
    id: String,
    primary_function: String,
}

#[derive(GraphQLUnion)]
#[graphql(description = "A Collection of things")]
pub enum Character {
    One(Human),
    Two(Droid),
}

#[derive(GraphQLUnion)]
#[graphql(Scalar = juniper::DefaultScalarValue)]
pub enum CharacterWithGeneric<T> {
    One(Human),
    Two(Droid),
    #[allow(dead_code)]
    #[graphql(ignore)]
    Hidden(T),
}

#[derive(GraphQLUnion)]
#[graphql(on Droid = CharacterCustomFn::as_droid)]
pub enum CharacterCustomFn {
    One(Human),
    #[graphql(ignore)]
    Two(Droid, usize, u8),
}

impl CharacterCustomFn {
    fn as_droid(&self, _: &()) -> Option<&Droid> {
        match self {
            Self::Two(droid, _, _) => Some(droid),
            _ => None,
        }
    }
}

#[derive(GraphQLUnion)]
pub enum CharacterCustomVariantFn {
    One(Human),
    #[graphql(with = CharacterCustomVariantFn::as_droid)]
    Two(Droid),
}

impl CharacterCustomVariantFn {
    fn as_droid(&self, _: &()) -> Option<&Droid> {
        match self {
            Self::Two(droid) => Some(droid),
            _ => None,
        }
    }
}

#[derive(GraphQLUnion)]
#[graphql(on Human = CharacterGenericStruct::as_human)]
#[graphql(on Droid = CharacterGenericStruct::as_droid)]
pub struct CharacterGenericStruct<T> {
    human: Human,
    droid: Droid,
    is_droid: bool,
    _gen: T,
}

impl<T> CharacterGenericStruct<T> {
    fn as_human(&self, _: &()) -> Option<&Human> {
        if self.is_droid {
            None
        } else {
            Some(&self.human)
        }
    }

    fn as_droid(&self, _: &()) -> Option<&Droid> {
        if self.is_droid {
            Some(&self.droid)
        } else {
            None
        }
    }
}


// Context Test
pub struct CustomContext {
    is_left: bool,
}

impl juniper::Context for CustomContext {}

#[derive(juniper::GraphQLObject)]
#[graphql(Context = CustomContext)]
pub struct HumanContext {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
#[graphql(Context = CustomContext)]
pub struct DroidContext {
    id: String,
    primary_function: String,
}

/// A Collection of things
#[derive(From, GraphQLUnion)]
#[graphql(Context = CustomContext)]
pub enum CharacterContext {
    One(HumanContext),
    Two(DroidContext),
}

// #[juniper::object] compatibility

pub struct HumanCompat {
    id: String,
    home_planet: String,
}

#[juniper::graphql_object]
impl HumanCompat {
    fn id(&self) -> &String {
        &self.id
    }

    fn home_planet(&self) -> &String {
        &self.home_planet
    }
}

pub struct DroidCompat {
    id: String,
    primary_function: String,
}

#[juniper::graphql_object]
impl DroidCompat {
    fn id(&self) -> &String {
        &self.id
    }

    fn primary_function(&self) -> &String {
        &self.primary_function
    }
}

#[derive(GraphQLUnion)]
#[graphql(Context = CustomContext)]
pub enum DifferentContext {
    A(DroidContext),
    B(Droid),
}

// NOTICE: this can not compile due to generic implementation of GraphQLType<__S>
// #[derive(GraphQLUnion)]
// pub enum CharacterCompatFail {
//     One(HumanCompat),
//     Two(DroidCompat),
// }

/// A Collection of things
#[derive(GraphQLUnion)]
#[graphql(scalar = juniper::DefaultScalarValue)]
pub enum CharacterCompat {
    One(HumanCompat),
    Two(DroidCompat),
}

pub struct Query;

#[juniper::graphql_object(
    Context = CustomContext,
)]
impl Query {
    fn context(&self, ctx: &CustomContext) -> CharacterContext {
        if ctx.is_left {
            HumanContext {
                id: "human-32".to_string(),
                home_planet: "earth".to_string(),
            }
            .into()
        } else {
            DroidContext {
                id: "droid-99".to_string(),
                primary_function: "run".to_string(),
            }
            .into()
        }
    }
}

#[tokio::test]
async fn test_derived_union_doc_macro() {
    assert_eq!(
        <Character as GraphQLType<DefaultScalarValue>>::name(&()),
        Some("Character")
    );

    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = Character::meta(&(), &mut registry);

    assert_eq!(meta.name(), Some("Character"));
    assert_eq!(
        meta.description(),
        Some(&"A Collection of things".to_string())
    );
}

#[tokio::test]
async fn test_derived_union_doc_string() {
    assert_eq!(
        <CharacterContext as GraphQLType<DefaultScalarValue>>::name(&()),
        Some("CharacterContext")
    );

    let mut registry: juniper::Registry = juniper::Registry::new(FnvHashMap::default());
    let meta = CharacterContext::meta(&(), &mut registry);

    assert_eq!(meta.name(), Some("CharacterContext"));
    assert_eq!(
        meta.description(),
        Some(&"A Collection of things".to_string())
    );
}

#[tokio::test]
async fn test_derived_union_left() {
    let doc = r#"
        {
            context {
                ... on HumanContext {
                    humanId: id
                    homePlanet
                }
                ... on DroidContext {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

    let schema = RootNode::new(
        Query,
        EmptyMutation::<CustomContext>::new(),
        EmptySubscription::<CustomContext>::new(),
    );

    assert_eq!(
        execute(
            doc,
            None,
            &schema,
            &Variables::new(),
            &CustomContext { is_left: true }
        )
        .await,
        Ok((
            Value::object(
                vec![(
                    "context",
                    Value::object(
                        vec![
                            ("humanId", Value::scalar("human-32".to_string())),
                            ("homePlanet", Value::scalar("earth".to_string())),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}

#[tokio::test]
async fn test_derived_union_right() {
    let doc = r#"
        {
            context {
                ... on HumanContext {
                    humanId: id
                    homePlanet
                }
                ... on DroidContext {
                    droidId: id
                    primaryFunction
                }
            }
        }"#;

    let schema = RootNode::new(
        Query,
        EmptyMutation::<CustomContext>::new(),
        EmptySubscription::<CustomContext>::new(),
    );

    assert_eq!(
        execute(
            doc,
            None,
            &schema,
            &Variables::new(),
            &CustomContext { is_left: false }
        )
        .await,
        Ok((
            Value::object(
                vec![(
                    "context",
                    Value::object(
                        vec![
                            ("droidId", Value::scalar("droid-99".to_string())),
                            ("primaryFunction", Value::scalar("run".to_string())),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                )]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}
