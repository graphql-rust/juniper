//! Checks that using a fragment of an implementation in an interface works okay.
//! See [#407](https://github.com/graphql-rust/juniper/issues/407) for details.

use juniper::{
    graphql_interface, graphql_object, graphql_vars, EmptyMutation, EmptySubscription,
    GraphQLObject,
};

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

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
struct Droid {
    id: String,
    serial_number: String,
}

#[graphql_object]
impl Query {
    fn characters() -> Vec<CharacterValue> {
        let human = Human {
            id: "1".into(),
            name: "Han Solo".into(),
        };
        let droid = Droid {
            id: "2".into(),
            serial_number: "234532545235".into(),
        };
        vec![Into::into(human), Into::into(droid)]
    }
}

type Schema = juniper::RootNode<Query, EmptyMutation, EmptySubscription>;

#[tokio::test]
async fn fragments_in_interface() {
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

    let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

    let (_, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
        .await
        .unwrap();
    assert_eq!(errors.len(), 0);

    let (_, errors) = juniper::execute_sync(query, None, &schema, &graphql_vars! {}, &()).unwrap();
    assert_eq!(errors.len(), 0);
}

#[tokio::test]
async fn inline_fragments_in_interface() {
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

    let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());

    let (_, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
        .await
        .unwrap();
    assert_eq!(errors.len(), 0);

    let (_, errors) = juniper::execute_sync(query, None, &schema, &graphql_vars! {}, &()).unwrap();
    assert_eq!(errors.len(), 0);
}
