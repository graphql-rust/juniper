use juniper::*;

struct Query;

#[graphql_object]
impl Query {
    fn artoo() -> Character {
        Character::Droid(Droid {
            id: 1,
            name: "R2-D2".to_owned(),
            sensor_color: "red".to_owned(),
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

type Schema = RootNode<'static, Query, EmptyMutation<()>, EmptySubscription<()>>;

#[tokio::test]
async fn test_fragment_on_interface() {
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

    let (res, errors) = execute(
        query,
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &(),
    )
    .await
    .unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(
        res,
        graphql_value!({
            "artoo": {"__typename": "Droid", "id": 1, "sensorColor": "red"}
        }),
    );

    let (res, errors) = execute_sync(
        query,
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &(),
    )
    .unwrap();

    assert_eq!(errors.len(), 0);
    assert_eq!(
        res,
        graphql_value!({
            "artoo": {"__typename": "Droid", "id": 1, "sensorColor": "red"}
        }),
    );
}
