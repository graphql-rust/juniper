# Tracing

Juniper has optional support for the [tracing] crate for instrumentation.

This feature is off by default and can be enabled via the `trace-sync`
feature to enable tracing of a sync code, `trace-async` feature to enable
tracing of an async code and subscriptions, and `trace` to enable 
functionality of both `trace-sync`and `trace-async` features.

!FILENAME Cargo.toml

```toml
[dependencies]
juniper = { version = "0.14.7", features = ["default", "trace"]}
tracing = "0.1.17"
tracing-subscriber = "0.2.9"
```

## Usage

```rust
# extern crate juniper;
extern crate tokio;
extern crate tracing;
extern crate tracing_subscriber;
use juniper::{EmptyMutation, EmptySubscription, RootNode, graphql_object, Variables};

#[derive(Clone, Copy, Debug)]
struct Query;

#[graphql_object]
impl Query {
    async fn foo() -> i32 {
        42
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<()>, EmptySubscription<()>>;


#[tokio::main]
async fn main() {
    // Set up the tracing subscriber.
    let subscriber = tracing_subscriber::fmt()
        // This enables standard env variables such as `RUST_LOG=trace`.
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        // This makes it so we can see all span events.
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default tracing subscriber failed");

    // Set up GraphQL information.
    let vars = Variables::new();
    let root = Schema::new(
        Query,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    // When run with `RUST_LOG=trace cargo run`, this should output traces /
    // span events to `stdout`.
    let query = "{ foo }";
    let (_, _errors) = juniper::execute(query, None, &root, &vars, &())
        .await
        .unwrap();
}
```

To trace how GraphQL object being resolved you need to enable one of tracing 
features and use `trace` argument on top-level `#[graphql_object]` and
`#[graphql_interface]` attributes or `#[graphql]` if used with `#[derive(GraphQLObject)]`.
`tracing` argument can be used with one of provided arguments:
`"sync"`, `"async"`, `"skip-all"` and `"complex"`.
 - Use `"sync"` to trace only synchronous part (struct fields and `fn`s).
 - Use `"async"` to trace only asynchronous part (`async fn`s) and
subscriptions.
 - Use `"complex"` to trace only fields marked with `#[tracing(complex)]`
 - Use `"skip-all"` to skip tracing of all fields.

In addition you can use `#[tracing(no_trace)]` with all variants above to
exclude field from tracing even if it belongs to traced group.

If resolving of certain field requires additional arguments (when used `fn`s or
`async fn`s) they also will be included in resulted trace (except `self` and
`Context`). You can use `skip` argument of `#[trace]` attribute, to skip some
arguments, similarly to the [`skip`] for `#[instrument]`

```rust
struct Catalog;

#[graphql_object]
impl Catalog {
    async fn products(filter: Filter, count: i32) -> Vec<Product> {
        // Some query
    }
}

struct User {
    id: i64
}

#[graphql_object]
impl User {
    fn id(&self) -> i64 {
        self.id
    }

    async fn friends(&self) -> Vec<i64> {
        // async database query 
    }
}
```

In case above both `filter` and `count` will be recorded in [`Span`] for
`Catalog::products(...)`.


## `#[tracing]` attribute

In most cases `#[tracing]` mimics behaviour of the `#[instrument]` attribute
from [tracing] crate and you could use it as a reference. With the only key
deference you should understand, it applied implicitly to all resolvers if the
`trace` feature is enabled.

[tracing]: https://crates.io/crates/tracing
[`skip`]: https://docs.rs/tracing/0.1.26/tracing/attr.instrument.html#skipping-fields
