//! Checks that interface field resolves okay on a union.
//! See [#798](https://github.com/graphql-rust/juniper/issues/798) for details.

use juniper::{
    graphql_interface, graphql_object, graphql_value, graphql_vars, EmptyMutation,
    EmptySubscription, GraphQLObject, GraphQLUnion, RootNode,
};

#[graphql_interface(for = [Human, Droid])]
trait Character {
    fn id(&self) -> &str;
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
struct Human {
    id: String,
    home_planet: String,
}

#[graphql_interface]
impl Character for Human {
    fn id(&self) -> &str {
        &self.id
    }
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
struct Droid {
    id: String,
    primary_function: String,
}

#[graphql_interface]
impl Character for Droid {
    fn id(&self) -> &str {
        &self.id
    }
}

#[derive(GraphQLUnion)]
enum FieldResult {
    Human(Human),
    Droid(Droid),
}

#[derive(Clone, Copy)]
enum Query {
    Human,
    Droid,
}

#[graphql_object]
impl Query {
    fn field(&self) -> FieldResult {
        match self {
            Self::Human => FieldResult::Human(Human {
                id: "human-32".to_owned(),
                home_planet: "earth".to_owned(),
            }),
            Self::Droid => FieldResult::Droid(Droid {
                id: "droid-99".to_owned(),
                primary_function: "run".to_owned(),
            }),
        }
    }
}

type Schema = RootNode<'static, Query, EmptyMutation, EmptySubscription>;

#[tokio::test]
async fn interface_inline_fragment_on_union() {
    let query = r#"
        query Query {
            field {
                __typename
                ... on Character {
                    id
                }
                ... on Human {
                    homePlanet
                }
                ... on Droid {
                    primaryFunction
                }
            }
        }
    "#;

    let schema = Schema::new(Query::Human, EmptyMutation::new(), EmptySubscription::new());
    let (res, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(
        res,
        graphql_value!({
            "field": {
                "__typename": "Human",
                "id": "human-32",
                "homePlanet": "earth",
            },
        }),
    );

    let schema = Schema::new(Query::Droid, EmptyMutation::new(), EmptySubscription::new());
    let (res, errors) =
        juniper::execute_sync(query, None, &schema, &graphql_vars! {}, &()).unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(
        res,
        graphql_value!({
            "field": {
                "__typename": "Droid",
                "id": "droid-99",
                "primaryFunction": "run",
            },
        }),
    );
}

#[tokio::test]
async fn interface_fragment_on_union() {
    let query = r#"
        query Query {
            field {
                __typename
                ... CharacterFragment
                ... on Human {
                    homePlanet
                }
                ... on Droid {
                    primaryFunction
                }
            }
        }

        fragment CharacterFragment on Character {
            id
        }
    "#;

    let schema = Schema::new(Query::Human, EmptyMutation::new(), EmptySubscription::new());
    let (res, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(
        res,
        graphql_value!({
            "field": {
                "__typename": "Human",
                "id": "human-32",
                "homePlanet": "earth",
            },
        }),
    );

    let schema = Schema::new(Query::Droid, EmptyMutation::new(), EmptySubscription::new());
    let (res, errors) =
        juniper::execute_sync(query, None, &schema, &graphql_vars! {}, &()).unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(
        res,
        graphql_value!({
            "field": {
                "__typename": "Droid",
                "id": "droid-99",
                "primaryFunction": "run",
            },
        }),
    );
}
