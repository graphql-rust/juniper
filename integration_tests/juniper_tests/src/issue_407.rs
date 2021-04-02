use juniper::*;

struct Query;

#[graphql_interface(for = [Human, Droid])]
trait Character {
    fn id(&self) -> &str;
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
struct Human {
    id: String,
    name: String,
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
    serial_number: String,
}

#[graphql_interface]
impl Character for Droid {
    fn id(&self) -> &str {
        &self.id
    }
}

#[graphql_object]
impl Query {
    fn characters() -> Vec<CharacterValue> {
        let human = Human {
            id: "1".to_string(),
            name: "Han Solo".to_string(),
        };
        let droid = Droid {
            id: "2".to_string(),
            serial_number: "234532545235".to_string(),
        };
        vec![Into::into(human), Into::into(droid)]
    }
}

type Schema = juniper::RootNode<'static, Query, EmptyMutation, EmptySubscription>;

#[tokio::test]
async fn test_fragments_in_interface() {
    let query = r#"
        query Query {
            characters {
                ...HumanFragment
                ...DroidFragment
            }
        }

        fragment HumanFragment on Human {
            name
        }

        fragment DroidFragment on Droid {
            serialNumber
        }
    "#;

    let (_, errors) = juniper::execute(
        query,
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &(),
    )
    .await
    .unwrap();
    assert_eq!(errors.len(), 0);

    let (_, errors) = juniper::execute_sync(
        query,
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &(),
    )
    .unwrap();
    assert_eq!(errors.len(), 0);
}

#[tokio::test]
async fn test_inline_fragments_in_interface() {
    let query = r#"
        query Query {
            characters {
                ...on Human {
                    ...HumanFragment
                }
                ...on Droid {
                    ...DroidFragment
                }
            }
        }

        fragment HumanFragment on Human {
            name
        }

        fragment DroidFragment on Droid {
            serialNumber
        }
    "#;

    let (_, errors) = juniper::execute(
        query,
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &(),
    )
    .await
    .unwrap();
    assert_eq!(errors.len(), 0);

    let (_, errors) = juniper::execute_sync(
        query,
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &(),
    )
    .unwrap();
    assert_eq!(errors.len(), 0);
}
