use juniper::{graphql_object, EmptyMutation, EmptySubscription, Executor, ScalarValue, Variables};

struct Query;

#[graphql_object]
impl Query {
    fn users<S: ScalarValue>(executor: &Executor<'_, '_, (), S>) -> Vec<User> {
        executor.look_ahead();

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
        executor.look_ahead();
        &self.city
    }
}

struct City {
    country: Country,
}

#[graphql_object]
impl City {
    fn country<S: ScalarValue>(&self, executor: &Executor<'_, '_, (), S>) -> &Country {
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

type Schema = juniper::RootNode<'static, Query, EmptyMutation<()>, EmptySubscription<()>>;

#[tokio::test]
async fn test_nested_fragments() {
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
}
