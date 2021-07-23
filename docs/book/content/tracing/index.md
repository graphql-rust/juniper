# Tracing

Juniper has optional support for the [tracing] crate for instrumentation.

This feature is off by default and can be enabled via the `tracing` feature.

!FILENAME Cargo.toml

```toml
[dependencies]
juniper = { version = "0.15.7", features = ["default", "tracing"]}
tracing = "0.1.26"
tracing-subscriber = "0.2.15"
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

struct Foo {
    value: i32,
}

#[graphql_object]
impl Foo {
    // Value resolving is pretty straightforward so we can skip tracing.
    #[tracing(no_trace)]
    fn value(&self) -> i32 {
        self.value
    }
    
    // Here we'll record span and it will have field with name "another" and value we passed.
    fn multiply_value(&self, another: i32) -> i32 {
        self.value * another
    }
    
    // Here we'll record span and it will have field with name "self.value"
    #[tracing(fields(self.value = self.value))]
    fn square_value(&self) -> i32 {
        self.value * self.value
    }
}

#[graphql_object]
impl Query {
    async fn foo() -> Foo {
        Foo { value: 42 }
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
    let query = r#"
        {
            foo { 
                value
                multiplyValues(another: 23)
                squareValue
            }
        }"#;
    let (_, _errors) = juniper::execute(query, None, &root, &vars, &())
        .await
        .unwrap();
}
```

## Skipping field resolvers

In certain scenarios you may want to skip tracing of some fields because it too
simple and straightforward that tracing preparations of this resolver would actually
take more time then execution. In this cases you can use `#[tracing(no_trace)]` to
completely disable tracing of this field resolver.

### Example

```rust
struct User {
    id: i32,
}

#[graphql_object(context = Context)]
impl User {
    #[tracing(no_trace)]
    fn id(&self) -> i32 {
        self.id
    }

    async fn friends(&self, context: Context) -> Vec<User> {
        // Some async query in which you're actually interested.
    }
}
```

Manually setting `#[tracing(no_traces)]` to avoid tracing of all, let's say for
example synchronous field resolvers is rather inefficient when you have GraphQL
object with too much fields. In this case you can use `tracing` argument on
top-level `#[graphql_object]`, `#[graphql_interface]` or `#[graphql]` (when it
used with `#[derive(GraphQLObject)]`) attributes to trace specific group of
fields or not to trace at all. `tracing` argument can be used with one of the
following arguments: `"sync"`, `"async"`, `"complex"` or `"skip-all"`.
 - Use `"sync"` to trace only synchronous part (struct fields and `fn`s).
 - Use `"async"` to trace only asynchronous part (`async fn`s) and
subscriptions.
 - Use `"complex"` to trace only fields marked with `#[tracing(complex)]`
 - Use `"skip-all"` to skip tracing of all fields.

**Note:** using of `trace = "sync"` with derived struct is no-op because all
resolvers within derived GraphQL object is considered to be synchronous, also
because of this `trace = "async"` will result in no traces.

In addition you can use `#[tracing(no_trace)]` with all variants above to
exclude field from tracing even if it belongs to traced group.

**Be careful when skipping trace as it can lead to bad structured span trees,
disabling of tracing on one level won't disable tracing in it's child methods.**

If resolving of certain field requires additional arguments (when used `fn`s or
`async fn`s) they also will be included in resulted trace (except `self` and
`Context`). You can use `skip` argument of `#[tracing]` attribute, to skip some
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
    id: i32
}

#[graphql_object]
impl User {
    fn id(&self) -> i32 {
        self.id
    }

    async fn friends(&self) -> Vec<i32> {
        // async database query 
    }
}
```

In case above both `filter` and `count` will be recorded in [`Span`] for
`Catalog::products(...)`. All fields will be recorded using their `Debug`
implementation, if your field doesn't implement `Debug` you should skip it
with `#[tracing(skip(<fields to skip>))]` if you still want to record it but
for some reason you don't want or can't use `Debug` trait, consider reintroducing
this field with `fields(field_name = some_value)`.


### Example
```rust
#[derive(Clone)]
struct NonDebug {
    important_field: String,
}

#[tracing(skip(non_debug), fields(non_debug = non_debug.important_field))]
fn my_query(&self, non_debug: NonDebug) -> i32 {
    24
}
```

## `#[tracing]` attribute

In most aspects `#[tracing]` mimics behaviour of the `#[instrument]` attribute
from [tracing] crate and you could use it as a reference. With the only key
deference you should understand, it applied implicitly to all resolvers if the
`tracing` feature is enabled.

[tracing]: https://crates.io/crates/tracing
[`skip`]: https://docs.rs/tracing/0.1.26/tracing/attr.instrument.html#skipping-fields
[`Span`]: https://docs.rs/tracing/0.1.26/tracing/struct.Span.html
