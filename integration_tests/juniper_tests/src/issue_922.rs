use juniper::*;

struct Query;

#[juniper::graphql_object]
impl Query {
    fn characters(executor: &Executor) -> Vec<CharacterValue> {
        executor.look_ahead();

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

    let res = match res {
        juniper::Value::Object(res) => res,
        _ => panic!("Object should be returned"),
    };
    let characters = res.get_field_value("characters");
    assert!(characters.is_some(), "No characters returned");

    if let juniper::Value::List(values) = characters.unwrap() {
        for obj in values {
            if let juniper::Value::Object(obj) = obj {
                assert!(obj.contains_field("id"), "id field should be present");
                assert!(obj.contains_field("name"), "name field should be present");
            } else {
                assert!(false, "List should contain value");
            }
        }
    } else {
        assert!(false, "List should be returned")
    }
}
