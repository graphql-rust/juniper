//! Checks that fields on interface fragment spreads resolve okay.
//! See [#922](https://github.com/graphql-rust/juniper/issues/922) for details.

use juniper::{
    graphql_interface, graphql_object, graphql_value, graphql_vars, EmptyMutation,
    EmptySubscription, GraphQLObject,
};

struct Query;

#[graphql_object]
impl Query {
    fn characters() -> Vec<CharacterValue> {
        vec![
            Into::into(Human {
                id: 0,
                name: "human-32".into(),
            }),
            Into::into(Droid {
                id: 1,
                name: "R2-D2".into(),
            }),
        ]
    }
}

#[graphql_interface(for = [Human, Droid])]
trait Character {
    fn id(&self) -> i32;

    fn name(&self) -> String;
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
struct Human {
    pub id: i32,
    pub name: String,
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
struct Droid {
    pub id: i32,
    pub name: String,
}

type Schema = juniper::RootNode<Query, EmptyMutation, EmptySubscription>;

#[tokio::test]
async fn object_fragment_on_interface() {
    let query = r#"
        query Query {
            characters {
                ...CharacterFragment
            }
        }

        fragment CharacterFragment on Character {
            __typename
            ... on Human {
                id
                name
            }
            ... on Droid {
                id
                name
            }
        }
    "#;

    let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

    let (res, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(
        res,
        graphql_value!({
            "characters": [
                {"__typename": "Human", "id": 0, "name": "human-32"},
                {"__typename": "Droid", "id": 1, "name": "R2-D2"},
            ],
        }),
    );

    let (res, errors) =
        juniper::execute_sync(query, None, &schema, &graphql_vars! {}, &()).unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(
        res,
        graphql_value!({
            "characters": [
                {"__typename": "Human", "id": 0, "name": "human-32"},
                {"__typename": "Droid", "id": 1, "name": "R2-D2"},
            ],
        }),
    );
}

#[tokio::test]
async fn interface_fragment_on_interface() {
    let query = r#"
        query Query {
            characters {
                ...CharacterFragment
            }
        }

        fragment CharacterFragment on Character {
            __typename
            ... on Character {
                id
                name
            }
        }
    "#;

    let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

    let (res, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(
        res,
        graphql_value!({
            "characters": [
                {"__typename": "Human", "id": 0, "name": "human-32"},
                {"__typename": "Droid", "id": 1, "name": "R2-D2"},
            ],
        }),
    );

    let (res, errors) =
        juniper::execute_sync(query, None, &schema, &graphql_vars! {}, &()).unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(
        res,
        graphql_value!({
            "characters": [
                {"__typename": "Human", "id": 0, "name": "human-32"},
                {"__typename": "Droid", "id": 1, "name": "R2-D2"},
            ],
        }),
    );
}
