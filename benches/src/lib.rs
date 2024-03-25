use juniper::{
    graphql_object, DefaultScalarValue, EmptyMutation, EmptySubscription, ExecutionError,
    FieldError, GraphQLEnum, GraphQLObject, RootNode, Value, Variables,
};

pub type QueryResult = Result<
    (
        Value<DefaultScalarValue>,
        Vec<ExecutionError<DefaultScalarValue>>,
    ),
    String,
>;

pub struct Context;

impl Context {
    fn new() -> Self {
        Self
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

#[derive(GraphQLObject)]
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
            username: "userx".into(),
            email: "userx@domain.com".into(),
            gender: Some(Gender::Female),
        }
    }
}

pub struct Query;

#[graphql_object(context = Context)]
impl Query {
    fn user_sync_instant(id: i32) -> Result<User, FieldError> {
        Ok(User::new(id))
    }

    fn users_sync_instant(ids: Option<Vec<i32>>) -> Result<Vec<User>, FieldError> {
        if let Some(ids) = ids {
            let users = ids.into_iter().map(User::new).collect();
            Ok(users)
        } else {
            Ok(vec![])
        }
    }

    async fn user_async_instant(id: i32) -> Result<User, FieldError> {
        Ok(User::new(id))
    }

    async fn users_async_instant(ids: Option<Vec<i32>>) -> Result<Vec<User>, FieldError> {
        if let Some(ids) = ids {
            let users = ids.into_iter().map(User::new).collect();
            Ok(users)
        } else {
            Ok(vec![])
        }
    }
}

pub fn new_schema() -> RootNode<Query, EmptyMutation<Context>, EmptySubscription<Context>> {
    RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new())
}

pub fn execute_sync(query: &str, vars: Variables) -> QueryResult {
    let root = new_schema();
    let ctx = Context::new();
    juniper::execute_sync(query, None, &root, &vars, &ctx).map_err(|e| format!("{e:?}"))
}

pub async fn execute(query: &str, vars: Variables) -> QueryResult {
    let root = new_schema();
    let ctx = Context::new();
    juniper::execute(query, None, &root, &vars, &ctx)
        .await
        .map_err(|e| format!("{e:?}"))
}
