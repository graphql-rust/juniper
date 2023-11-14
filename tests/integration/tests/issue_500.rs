//! Checks that using nested fragments works okay.
//! See [#500](https://github.com/graphql-rust/juniper/issues/500) for details.

use juniper::{
    graphql_object, graphql_vars, EmptyMutation, EmptySubscription, Executor, ScalarValue,
};

struct Query;

#[graphql_object]
impl Query {
    fn users<S: ScalarValue>(executor: &Executor<'_, '_, (), S>) -> Vec<User> {
        assert_eq!(executor.look_ahead().field_name(), "users");
        executor.look_ahead().children();

        vec![User {
            city: City {
                country: Country { id: 1 },
            },
        }]
    }
}

struct User {
    city: City,
}

#[graphql_object]
impl User {
    fn city<S: ScalarValue>(&self, executor: &Executor<'_, '_, (), S>) -> &City {
        assert_eq!(executor.look_ahead().field_name(), "city");
        executor.look_ahead().children();
        &self.city
    }
}

struct City {
    country: Country,
}

#[graphql_object]
impl City {
    fn country<S: ScalarValue>(&self, executor: &Executor<'_, '_, (), S>) -> &Country {
        assert_eq!(executor.look_ahead().field_name(), "country");
        executor.look_ahead().children();
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

type Schema = juniper::RootNode<'static, Query, EmptyMutation<()>, EmptySubscription<()>>;

#[tokio::test]
async fn nested_fragments() {
    let query = r#"
        query Query {
            users {
                ...UserFragment
            }
        }

        fragment UserFragment on User {
            city {
                ...CityFragment
            }
        }

        fragment CityFragment on City {
            country {
                ...CountryFragment
            }
        }

        fragment CountryFragment on Country {
            id
        }
    "#;

    let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());
    let (_, errors) = juniper::execute(query, None, &schema, &graphql_vars! {}, &())
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
}
