//!

use juniper::{
    graphql_object, DefaultScalarValue, EmptyMutation, EmptySubscription, ExecutionError,
    FieldError, GraphQLEnum, IntrospectionFormat, Value, Variables,
};

pub type QueryResult = Result<
    (
        Value<DefaultScalarValue>,
        Vec<ExecutionError<DefaultScalarValue>>,
    ),
    String,
>;

pub struct Context {}

impl Context {
    fn new() -> Self {
        Self {}
    }
}

impl juniper::Context for Context {}

#[derive(GraphQLEnum)]
pub enum Gender {
    Male,
    Female,
    Other,
}

#[derive(GraphQLEnum)]
pub enum UserKind {
    SuperAdmin,
    Admin,
    Moderator,
    User,
    Guest,
}

#[derive(juniper::GraphQLObject)]
pub struct User {
    pub id: i32,
    pub kind: UserKind,
    pub username: String,
    pub email: String,
    pub gender: Option<Gender>,
}

impl User {
    fn new(id: i32) -> Self {
        Self {
            id,
            kind: UserKind::Admin,
            username: "userx".to_string(),
            email: "userx@domain.com".to_string(),
            gender: Some(Gender::Female),
        }
    }
}

pub struct Query;

#[graphql_object(Context = Context)]
impl Query {
    async fn user_instant(id: i32) -> Result<User, FieldError> {
        Ok(User::new(id))
    }

    async fn users_instant(ids: Option<Vec<i32>>) -> Result<Vec<User>, FieldError> {
        if let Some(ids) = ids {
            let users = ids.into_iter().map(User::new).collect();
            Ok(users)
        } else {
            Ok(vec![])
        }
    }
}

pub fn new_schema(
) -> juniper::RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>> {
    juniper::RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new())
}

pub async fn execute(
    query: &str,
    vars: Variables,
) -> (Value, Vec<ExecutionError<DefaultScalarValue>>) {
    let root = new_schema();
    let ctx = Context::new();
    juniper::execute(query, None, &root, &vars, &ctx)
        .await
        .map_err(|e| format!("{:?}", e))
        .unwrap()
}

pub async fn introspect(
    format: IntrospectionFormat,
) -> (Value, Vec<ExecutionError<DefaultScalarValue>>) {
    let root = new_schema();
    let ctx = Context::new();
    juniper::introspect(&root, &ctx, format)
        .await
        .map_err(|e| format!("{:?}", e))
        .unwrap()
}
