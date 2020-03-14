// Original author of this test is <https://github.com/davidpdrsn>.
use juniper::*;

use futures;

pub struct Context;

impl juniper::Context for Context {}

pub struct Query;

#[graphql_object(
    Context = Context
)]
impl Query {
    fn users(exec: &Executor) -> Vec<User> {
        let lh = exec.look_ahead();
        assert_eq!(lh.field_name(), "users");
        vec![User]
    }

    fn countries(exec: &Executor) -> Vec<Country> {
        let lh = exec.look_ahead();
        assert_eq!(lh.field_name(), "countries");
        vec![Country]
    }
}

#[derive(Clone)]
pub struct User;

#[graphql_object(
    Context = Context
)]
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

type Schema = juniper::RootNode<'static, Query, EmptyMutation<Context>>;

#[tokio::test]
async fn users() {
    let ctx = Context;

    let query = r#"{ users { id } }"#;

    let (_, errors) = juniper::execute(
        query,
        None,
        &Schema::new(Query, EmptyMutation::<Context>::new()),
        &juniper::Variables::new(),
        &ctx,
    )
    .await
    .unwrap();

    assert_eq!(errors.len(), 0);
}

#[tokio::test]
async fn countries() {
    let ctx = Context;

    let query = r#"{ countries { id } }"#;

    let (_, errors) = juniper::execute(
        query,
        None,
        &Schema::new(Query, EmptyMutation::new()),
        &juniper::Variables::new(),
        &ctx,
    )
    .await
    .unwrap();

    assert_eq!(errors.len(), 0);
}

#[tokio::test]
async fn both() {
    let ctx = Context;

    let query = r#"
    {
        countries { id }
        users { id }
    }
    "#;

    let (_, errors) = juniper::execute(
        query,
        None,
        &Schema::new(Query, EmptyMutation::<Context>::new()),
        &juniper::Variables::new(),
        &ctx,
    )
    .await
    .unwrap();

    assert_eq!(errors.len(), 0);
}

#[tokio::test]
async fn both_in_different_order() {
    let ctx = Context;

    let query = r#"
    {
        users { id }
        countries { id }
    }
    "#;

    let (_, errors) = juniper::execute(
        query,
        None,
        &Schema::new(Query, EmptyMutation::<Context>::new()),
        &juniper::Variables::new(),
        &ctx,
    )
    .await
    .unwrap();

    assert_eq!(errors.len(), 0);
}
