extern crate juniper;
extern crate tokio;
extern crate tracing;
extern crate tracing_subscriber;

use juniper::{
    graphql_object, EmptyMutation, EmptySubscription, FieldError, GraphQLEnum, RootNode, Variables,
};
use tracing::{trace_span, Instrument as _};
use tracing_subscriber::EnvFilter;

#[derive(Clone, Copy, Debug)]
struct Context;
impl juniper::Context for Context {}

#[derive(Clone, Copy, Debug, GraphQLEnum)]
enum UserKind {
    Admin,
    User,
    Guest,
}

#[derive(Clone, Debug)]
struct User {
    id: i32,
    kind: UserKind,
    name: String,
}

#[graphql_object(Context = Context)]
impl User {
    fn id(&self) -> i32 {
        self.id
    }

    fn kind(&self) -> UserKind {
        self.kind
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn friends(&self) -> Vec<User> {
        vec![]
    }
}

#[derive(Clone, Copy, Debug)]
struct Query;

#[graphql_object(Context = Context)]
impl Query {
    async fn users() -> Vec<User> {
        vec![User {
            id: 1,
            kind: UserKind::Admin,
            name: "user1".into(),
        }]
    }

    fn bob() -> User {
        User {
            id: 1,
            kind: UserKind::Admin,
            name: "Bob".into(),
        }
    }

    /// Double the provided number.
    async fn double(x: i32) -> Result<i32, FieldError> {
        Ok(x * 2)
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;

fn schema() -> Schema {
    Schema::new(
        Query,
        EmptyMutation::<Context>::new(),
        EmptySubscription::<Context>::new(),
    )
}

#[tokio::main]
async fn main() {
    // A builder for `FmtSubscriber`.
    let subscriber = tracing_subscriber::fmt()
        // This enables standard env variables such as `RUST_LOG=trace`.
        .with_env_filter(EnvFilter::from_default_env())
        // This makes it so we can see all span events.
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("no global subscriber has been set");

    let ctx = Context {};
    let vars = Variables::new();
    let root = schema();

    // When run with `RUST_LOG=trace cargo run`, this should output to `stdout`.
    let query = "{ users { id } }";
    let (_, _errors) = juniper::execute(query, None, &root, &vars, &ctx)
        .await
        .unwrap();

    // When run with `RUST_LOG=trace cargo run`, this should output to `stdout`.
    // Note that there is a top-level span of "doubling{42}" as it was set
    // here. This is useful to attach context to each call to `execute`.
    let query = "{ double(x: 42) }";
    let (_, _errors) = juniper::execute(query, None, &root, &vars, &ctx)
        .instrument(trace_span!("doubling", "{}", 42))
        .await
        .unwrap();

    // You can also trace sync execution.
    // This should output a validation error in the middle of other spans.
    let query = "{ bob { field_that_does_not_exist } }";
    let _ = juniper::execute_sync(query, None, &root, &vars, &ctx);
}
