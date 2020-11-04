use juniper::*;

struct Query;

#[juniper::graphql_object]
impl Query {
    fn users(executor: &Executor) -> Vec<User> {
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

#[juniper::graphql_object]
impl User {
    fn city(&self, executor: &Executor) -> &City {
        executor.look_ahead();
        &self.city
    }
}

struct City {
    country: Country,
}

#[juniper::graphql_object]
impl City {
    fn country(&self, executor: &Executor) -> &Country {
        executor.look_ahead();
        &self.country
    }
}

struct Country {
    id: i32,
}

#[juniper::graphql_object]
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
