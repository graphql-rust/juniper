// Original author of this test is <https://github.com/davidpdrsn>.

use juniper::{
    graphql_object, EmptyMutation, EmptySubscription, Executor, LookAheadMethods as _, RootNode,
    ScalarValue, Variables,
};

pub struct Context;

impl juniper::Context for Context {}

pub struct Query;

#[graphql_object(context = Context)]
impl Query {
    fn users<__S: ScalarValue>(executor: &Executor<'_, '_, Context, __S>) -> Vec<User> {
        let lh = executor.look_ahead();
        assert_eq!(lh.field_name(), "users");
        vec![User]
    }

    fn countries<__S: ScalarValue>(executor: &Executor<'_, '_, Context, __S>) -> Vec<Country> {
        let lh = executor.look_ahead();
        assert_eq!(lh.field_name(), "countries");
        vec![Country]
    }
}

#[derive(Clone)]
pub struct User;

#[graphql_object(context = Context)]
impl User {
    fn id() -> i32 {
        1
    }
}

#[derive(Clone)]
pub struct Country;

#[graphql_object]
impl Country {
    fn id() -> i32 {
        2
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;

#[tokio::test]
async fn users() {
    let query = "{ users { id } }";

    let schema = Schema::new(
        Query,
        EmptyMutation::<Context>::new(),
        EmptySubscription::<Context>::new(),
    );
    let (_, errors) = juniper::execute(query, None, &schema, &Variables::new(), &Context)
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
}

#[tokio::test]
async fn countries() {
    let query = "{ countries { id } }";

    let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());
    let (_, errors) = juniper::execute(query, None, &schema, &Variables::new(), &Context)
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
}

#[tokio::test]
async fn both() {
    let query = "{
        countries { id }
        users { id }
    }";

    let schema = Schema::new(
        Query,
        EmptyMutation::<Context>::new(),
        EmptySubscription::<Context>::new(),
    );
    let (_, errors) = juniper::execute(query, None, &schema, &Variables::new(), &Context)
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
}

#[tokio::test]
async fn both_in_different_order() {
    let query = "{
        users { id }
        countries { id }
    }";

    let schema = Schema::new(
        Query,
        EmptyMutation::<Context>::new(),
        EmptySubscription::<Context>::new(),
    );
    let (_, errors) = juniper::execute(query, None, &schema, &Variables::new(), &Context)
        .await
        .unwrap();

    assert_eq!(errors.len(), 0);
}
