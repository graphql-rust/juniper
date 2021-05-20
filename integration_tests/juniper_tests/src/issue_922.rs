use juniper::*;

struct Query;

#[juniper::graphql_object]
impl Query {
    fn characters() -> Vec<CharacterValue> {
        vec![
            Into::into(Human {
                id: 0,
                name: "human-32".to_owned(),
            }),
            Into::into(Droid {
                id: 1,
                name: "R2-D2".to_owned(),
            }),
        ]
    }
}

#[juniper::graphql_interface(for = [Human, Droid])]
trait Character {
    fn id(&self) -> i32;

    fn name(&self) -> String;
}

#[derive(juniper::GraphQLObject)]
#[graphql(impl = CharacterValue)]
struct Human {
    pub id: i32,
    pub name: String,
}

#[juniper::graphql_interface]
impl Character for Human {
    fn id(&self) -> i32 {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

#[derive(juniper::GraphQLObject)]
#[graphql(impl = CharacterValue)]
struct Droid {
    pub id: i32,
    pub name: String,
}

#[juniper::graphql_interface]
impl Character for Droid {
    fn id(&self) -> i32 {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

type Schema = juniper::RootNode<'static, Query, EmptyMutation<()>, EmptySubscription<()>>;

#[tokio::test]
async fn test_fragment_on_interface() {
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

    let (res, errors) = juniper::execute(
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
            "characters": [
                {"__typename": "Human", "id": 0, "name": "human-32"},
                {"__typename": "Droid", "id": 1, "name": "R2-D2"},
            ],
        }),
    );

    let (res, errors) = juniper::execute_sync(
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
            "characters": [
                {"__typename": "Human", "id": 0, "name": "human-32"},
                {"__typename": "Droid", "id": 1, "name": "R2-D2"},
            ],
        }),
    );
}
