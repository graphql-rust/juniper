//! Checks that spreading untyped union fragment work okay.
//! See [#945](https://github.com/graphql-rust/juniper/issues/945) for details.

use juniper::{
    graphql_object, graphql_value, graphql_vars, EmptyMutation, EmptySubscription, GraphQLObject,
    GraphQLUnion,
};

struct Query;

#[graphql_object]
impl Query {
    fn artoo() -> Character {
        Character::Droid(Droid {
            id: 1,
            name: "R2-D2".into(),
            sensor_color: "red".into(),
        })
    }
}

#[derive(GraphQLUnion)]
enum Character {
    Droid(Droid),
    #[allow(dead_code)]
    Human(Human),
}

#[derive(GraphQLObject)]
struct Human {
    pub id: i32,
    pub name: String,
    pub eye_color: String,
}

#[derive(GraphQLObject)]
struct Droid {
    pub id: i32,
    pub name: String,
    pub sensor_color: String,
}

type Schema = juniper::RootNode<Query, EmptyMutation, EmptySubscription>;

#[tokio::test]
async fn fragment_on_union() {
    let query = r#"
        query Query {
            artoo {
                ...CharacterFragment
            }
        }

        fragment CharacterFragment on Character {
            __typename
            ... on Human {
                id
                eyeColor
            }
            ... on Droid {
                id
                sensorColor
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
            "artoo": {"__typename": "Droid", "id": 1, "sensorColor": "red"},
        }),
    );

    let (res, errors) =
        juniper::execute_sync(query, None, &schema, &graphql_vars! {}, &()).unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(
        res,
        graphql_value!({
            "artoo": {"__typename": "Droid", "id": 1, "sensorColor": "red"},
        }),
    );
}
