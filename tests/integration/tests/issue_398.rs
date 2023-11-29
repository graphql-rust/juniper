//! Checks that `executor.look_ahead()` on a fragment with nested type works okay.
//! See [#398](https://github.com/graphql-rust/juniper/issues/398) for details.
//!
//! Original author of this test is [@davidpdrsn](https://github.com/davidpdrsn).

use juniper::{
    graphql_object, graphql_vars, EmptyMutation, EmptySubscription, Executor, RootNode, ScalarValue,
};

struct Query;

#[graphql_object]
impl Query {
    fn users<S: ScalarValue>(executor: &Executor<'_, '_, (), S>) -> Vec<User> {
        // This doesn't cause a panic.
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
    fn country<S: ScalarValue>(&self, executor: &Executor<'_, '_, (), S>) -> &Country {
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
async fn lookahead_from_fragment_with_nested_type() {
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
        &graphql_vars! {},
        &(),
    )
    .await
    .unwrap();
}
