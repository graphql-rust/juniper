//! Tests for `#[graphql_interface]` macro.

use juniper::{execute, graphql_object, graphql_interface, graphql_value, DefaultScalarValue, EmptyMutation, EmptySubscription, GraphQLObject, GraphQLType, RootNode, ScalarValue, Variables};

#[derive(GraphQLObject)]
#[graphql(impl = dyn Character)]
struct Human {
    id: String,
    home_planet: String,
}

#[graphql_interface]
impl Character for Human {
    //#[graphql_interface]
    async fn id(&self) -> &str {
        &self.id
    }
}

// ------------------------------------------

#[derive(GraphQLObject)]
#[graphql(impl = dyn Character)]
struct Droid {
    id: String,
    primary_function: String,
}

#[graphql_interface]
impl Character for Droid {
    async fn id(&self) -> &str {
        &self.id
    }

    fn as_droid(&self) -> Option<&Droid> {
        Some(self)
    }
}

// ------------------------------------------

#[graphql_interface(for = [Human, Droid])]
trait Character {
    async fn id(&self) -> &str;

    //#[graphql_interface(downcast)]
    fn as_droid(&self) -> Option<&Droid> { None }
}

// ------------------------------------------

fn schema<'q, C, S, Q>(query_root: Q) -> RootNode<'q, Q, EmptyMutation<C>, EmptySubscription<C>, S>
    where
        Q: GraphQLType<S, Context = C, TypeInfo = ()> + 'q,
        S: ScalarValue + 'q,
{
    RootNode::new(
        query_root,
        EmptyMutation::<C>::new(),
        EmptySubscription::<C>::new(),
    )
}

mod poc {
    use super::*;

    type DynCharacter<'a, S = DefaultScalarValue> = dyn Character<S, Context=(), TypeInfo=()> + 'a + Send + Sync;

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

    #[tokio::test]
    async fn resolves_id_for_human() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"id": "human-32"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_for_droid() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"id": "droid-99"}}),
                vec![],
            )),
        );
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
            execute(DOC, None, &schema, &Variables::new(), &()).await,
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
        //panic!("🔬 {:#?}", schema.schema);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }
}
