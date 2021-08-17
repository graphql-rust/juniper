extern crate juniper;
extern crate tokio;
extern crate tracing;
extern crate tracing_subscriber;

use juniper::{
    graphql_object, EmptyMutation, EmptySubscription, FieldError, GraphQLEnum, GraphQLObject,
    RootNode, Variables,
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
    // `id` can be resolved pretty straight-forward so we mark it with `ignore`
    #[graphql(tracing(ignore))]
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

#[derive(Clone, Debug)]
struct SyncTracedUser {
    id: i32,
    kind: UserKind,
    name: String,
}

// Only sync `fn`s will be traced if they're not marked with `#[graphql(tracing(ignore))]`.
#[graphql_object(Context = Context)]
#[tracing(sync)]
impl SyncTracedUser {
    // Won't be traced because it's marked with `ignore`
    #[graphql(tracing(ignore))]
    fn id(&self) -> i32 {
        self.id
    }

    // Won't be traced because it's `async fn`
    async fn kind(&self) -> UserKind {
        self.kind
    }

    // The only resolver that will be traced
    fn name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Clone, Debug, GraphQLObject)]
#[tracing(only)]
struct ComplexDerivedUser {
    // This shouldn't be traced because it's not marked with `tracing(only)`
    id: i32,
    // This is the only field that will be traced because it's marked with `tracing(only)`
    #[graphql(tracing(only))]
    kind: UserKind,
    // This also shouldn't be traced because there is no `tracing(only)`.
    name: String,
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

    /// Create guest user with the given `id` and `name`.
    #[instrument(skip(id))] // Here we skip `id` from being recorded into spans fields
    fn guest(id: i32, name: String) -> User {
        User {
            id,
            kind: UserKind::Guest,
            name,
        }
    }

    fn sync_user() -> SyncTracedUser {
        SyncTracedUser {
            id: 1,
            kind: UserKind::User,
            name: "Charlie".into(),
        }
    }

    fn complex_derived() -> ComplexDerivedUser {
        ComplexDerivedUser {
            id: 42,
            kind: UserKind::Admin,
            name: "Dave".into(),
        }
    }

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

    // If you use tracing with something like `jaeger_opentracing` span with name
    // 'Query.guest' which has field 'name' with value '"Not Bob"' and 1 child span
    // 'User.kind'. There won't be traces to 'User.id' because we marked it with
    // `#[graphql(tracing(ignore))]`
    let query = "{ guest(id: 1, name: \"Not Bob\") { id  kind} }";
    let (_, _errors) = juniper::execute(query, None, &root, &vars, &ctx)
        .await
        .unwrap();

    // Here you'll see span 'Query.syncUser' with one child span
    // 'SyncTracedUser.kind' because it's synchronous and not marked with
    // `#[graphql(tracing(ignore))]`.
    let query = "{ syncUser { id name kind} }";
    let (_, _errors) = juniper::execute(query, None, &root, &vars, &ctx)
        .await
        .unwrap();

    // Here you'll see span 'Query.complexUser' with one child span
    // 'ComplexDerivedUser.kind' because it's marked with
    // `#[graphql(tracing(only))]`.
    let query = "{ complexUser { id name kind }}";
    let (_, _errors) = juniper::execute(query, None, &root, &vars, &ctx)
        .await
        .unwrap();
}
