use juniper::{
    graphql_value, guards::MaybeOwned, EmptyMutation, EmptySubscription, FieldError, RootNode,
    Value,
};
use warp::{http::Response, Filter};

type Ctx = Context<String, Roles, Permissions>;

type Schema = RootNode<'static, Query, EmptyMutation<Ctx>, EmptySubscription<Ctx>>;

fn schema() -> Schema {
    RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new())
}

#[tokio::main]
async fn main() {
    ::std::env::set_var("RUST_LOG", "warp_server");
    env_logger::init();

    let log = warp::log("warp_server");

    let homepage = warp::path::end().map(|| {
        Response::builder()
            .header("content-type", "text/html")
            .body(format!(
                "<html><h1>juniper_warp</h1><div>visit <a href=\"/graphiql\">/graphiql</a></html>"
            ))
    });

    log::info!("Listening on 127.0.0.1:8080");

    let state = warp::any().map(move || Context {
        database: std::sync::Arc::new(example()),
        user: Some("alice".to_string()),
    });
    let graphql_filter = juniper_warp::make_graphql_filter(schema(), state.boxed());

    warp::serve(
        warp::get()
            .and(warp::path("graphiql"))
            .and(juniper_warp::graphiql_filter("/graphql", None))
            .or(homepage)
            .or(warp::path("graphql").and(graphql_filter))
            .with(log),
    )
    .run(([127, 0, 0, 1], 8080))
    .await
}

use std::collections::HashMap;

struct Database<U, R, P> {
    roles: HashMap<U, Vec<R>>,
    mapping: HashMap<R, Vec<P>>,
}

impl<U, R, P> Database<U, R, P>
where
    R: Eq + std::hash::Hash,
    U: Eq + std::hash::Hash,
    P: PartialEq,
{
    pub fn new(roles: HashMap<U, Vec<R>>, mapping: HashMap<R, Vec<P>>) -> Self {
        Self { roles, mapping }
    }

    pub fn has(&self, user: &U, permission: &P) -> bool {
        self.roles
            .get(user)
            .map(|roles| {
                roles.iter().any(|role| {
                    self.mapping
                        .get(role)
                        .map(|permissions| permissions.contains(permission))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
enum AuthResult<U, P> {
    MissingAuthentication,
    WrongPermission(U, P),
}

impl<S, U, P> juniper::IntoFieldError<S> for AuthResult<U, P>
where
    U: std::fmt::Debug,
    P: std::fmt::Debug,
    S: juniper::ScalarValue,
{
    fn into_field_error(self) -> juniper::FieldError<S> {
        match self {
            AuthResult::MissingAuthentication => {
                let v: Value<S> = graphql_value!({
                    "type": "AUTHENTICATION"
                });
                FieldError::new("Not Found", v)
            }
            AuthResult::WrongPermission(user, perm) => {
                let v: Value<S> = graphql_value!({
                    "type": "PERMISSION",
                    "user": { format!("{:?}", user) },
                    "permission": { format!("{:?}", perm) },
                });
                FieldError::new("Not Found", v)
            }
        }
    }
}

#[derive(Clone)]
struct Context<U, R, P> {
    database: std::sync::Arc<Database<U, R, P>>,
    user: Option<U>,
}

impl<U, R, P> juniper::Context for Context<U, R, P> {}

#[derive(Debug, Clone)]
struct HasPermission<P>(pub P);

impl<S, U, R, P> juniper::GraphQLGuard<S, Context<U, R, P>> for HasPermission<P>
where
    S: juniper::ScalarValue,
    P: Clone + std::fmt::Debug + Send + Sync + PartialEq + 'static,
    U: Clone + std::fmt::Debug + Send + Sync + Eq + std::hash::Hash + 'static,
    R: Eq + std::hash::Hash + Send + Sync + 'static,
{
    type Error = AuthResult<U, P>;

    type CtxOut = Context<U, R, P>;

    fn protected<'a>(
        &'a self,
        ctx: MaybeOwned<'a, Context<U, R, P>>,
    ) -> juniper::BoxFuture<Result<MaybeOwned<'a, Self::CtxOut>, Self::Error>> {
        let val = match ctx.as_ref().user {
            Some(ref user) if ctx.as_ref().database.has(&user, &self.0) => Ok(ctx),
            Some(ref user) => Err(AuthResult::WrongPermission(user.clone(), self.0.clone())),
            None => Err(AuthResult::MissingAuthentication),
        };
        futures::future::FutureExt::boxed(futures::future::ready(val))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Permissions {
    ReadSecret,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum Roles {
    Admin,
    User,
}

struct Query;

#[juniper::graphql_object(Context = Ctx)]
impl Query {
    async fn public(&self, ctx: &Ctx) -> String {
        "Hello World!".to_string()
    }

    #[graphql(Guard = "HasPermission(Permissions::ReadSecret)")]
    async fn private(&self, ctx: &Ctx) -> String {
        "Top secret".to_string()
    }

    #[graphql(
        Context = AdvancedContext,
        Guard = "Change(Permissions::ReadSecret, \"Hi\".to_string())"
    )]
    async fn diff(&self, ctx: &AdvancedContext) -> String {
        ctx.notice.to_string()
    }
}

fn example() -> Database<String, Roles, Permissions> {
    Database {
        roles: vec![
            ("bob".to_string(), vec![Roles::User]),
            ("alice".to_string(), vec![Roles::Admin]),
        ]
        .into_iter()
        .collect(),
        mapping: vec![
            (Roles::Admin, vec![Permissions::ReadSecret]),
            (Roles::User, vec![]),
        ]
        .into_iter()
        .collect(),
    }
}

impl juniper::Context for AdvancedContext {}

struct AdvancedContext {
    old: Ctx,
    notice: String,
}

struct Change<P>(pub P, pub String);

impl<S> juniper::GraphQLGuard<S, Ctx> for Change<Permissions>
where
    S: juniper::ScalarValue,
{
    type Error = AuthResult<String, Permissions>;

    type CtxOut = AdvancedContext;

    fn protected<'a>(
        &'a self,
        ctx: MaybeOwned<'a, Ctx>,
    ) -> juniper::BoxFuture<Result<MaybeOwned<'a, Self::CtxOut>, Self::Error>> {
        let val = match ctx.as_ref().user {
            Some(ref user) if ctx.as_ref().database.has(&user, &self.0) => Ok(AdvancedContext {
                old: Clone::clone(ctx.as_ref()),
                notice: self.1.clone(),
            }
            .into()),
            Some(ref user) => Err(AuthResult::WrongPermission(user.clone(), self.0.clone())),
            None => Err(AuthResult::MissingAuthentication),
        };
        futures::future::FutureExt::boxed(futures::future::ready(val))
    }
}
