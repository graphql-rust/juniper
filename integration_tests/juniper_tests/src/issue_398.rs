// Original author of this test is <https://github.com/davidpdrsn>.

use juniper::{graphql_object, EmptyMutation, EmptySubscription, RootNode, Variables};

struct Query;

#[graphql_object]
impl Query {
    fn users(executor: &Executor) -> Vec<User> {
        // This doesn't cause a panic
        executor.look_ahead();

        vec![User {
            country: Country { id: 1 },
        }]
    }
}

struct User {
    country: Country,
}

#[graphql_object]
impl User {
    fn country(&self, executor: &Executor) -> &Country {
        // This panics!
        executor.look_ahead();

        &self.country
    }
}

struct Country {
    id: i32,
}

#[graphql_object]
impl Country {
    fn id(&self) -> i32 {
        self.id
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<()>, EmptySubscription<()>>;

#[tokio::test]
async fn test_lookahead_from_fragment_with_nested_type() {
    let _ = juniper::execute(
        r#"
            query Query {
                users {
                    ...userFields
                }
            }

            fragment userFields on User {
                country {
                    id
                }
            }
        "#,
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &(),
    )
    .await
    .unwrap();
}
