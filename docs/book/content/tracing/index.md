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
    // Value resolving is pretty straight-forward so we can skip tracing.
    #[graphql(tracing(ignore))]
    fn value(&self) -> i32 {
        self.value
    }
    
    // Here we'll record span and it will have field with name "another" and value we passed.
    fn multiply_values(&self, another: i32) -> i32 {
        self.value * another
    }
    
    // Here we'll record span and it will have field with name "self.value"
    #[instrument(fields(self.value = self.value))]
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
simple and straight-forward, that tracing preparations of this resolver would actually
take more time then execution. In this cases you can use `tracing(ignore)` argument of
`#[graphql]` attribute to completely disable tracing of this field resolver.

### Example

```rust
# extern crate juniper;
# use juniper::graphql_object;
# fn main() {}
#
# struct Context;
# impl juniper::Context for Context {}

struct User {
    id: i32,
}

#[graphql_object(context = Context)]
impl User {
    #[graphql(tracing(ignore))]
    fn id(&self) -> i32 {
        self.id
    }

    async fn friends(&self, context: &Context) -> Vec<User> {
        // Some async query in which you're actually interested.
#       unimplemented!()
    }
}
```

Manually setting `#[graphql(tracing(ignore))]` to avoid tracing of all, let's say for
example synchronous field resolvers is rather inefficient when you have GraphQL
object with too much fields. In this case you can use `tracing` argument on
top-level `#[graphql_object]`, `#[graphql_interface]` or `#[graphql]` (when it
used with `#[derive(GraphQLObject)]`) attributes to trace specific group of
fields or not to trace at all. `tracing` argument can be used with one of the
following arguments: `sync`, `async`, `only` or `skip_all`.
 - Use `sync` to trace only synchronous part (struct fields and `fn`s).
 - Use `async` to trace only asynchronous part (`async fn`s) and
subscriptions.
 - Use `only` to trace only fields marked with `#[graphql(tracing(only))]`
 - Use `skip_all` to skip tracing of all fields.

**Note:** using of `tracing(sync)` with derived struct is no-op because all
resolvers within derived GraphQL object is considered to be synchronous, also
because of this `tracing(async)` will result in no traces.

In addition you can use `#[graphql(tracing(ignore))]` with `skip` and `async`
variants to exclude field from tracing even if it belongs to traced group.

**Be careful when skipping trace as it can lead to bad structured span trees,
disabling of tracing on one level won't disable tracing in it's child methods.**

If resolving of certain field requires additional arguments (when used `fn`s or
`async fn`s) they also will be included in resulted trace (except `self` and
`Context`).

```rust
# extern crate juniper;
# use juniper::{graphql_object, GraphQLObject};
#
# fn main() {}
#
# struct Context;
# impl juniper::Context for Context {}
#
# type Filter = i32;
#
# #[derive(GraphQLObject)]
# struct Product {
#     id: i32   
# }
#
# struct Catalog;
#[graphql_object]
impl Catalog {
    async fn products(filter: Filter, count: i32) -> Vec<Product> {
        // Some query
# unimplemented!()
    }
}
```

In example above both `filter` and `count` of `products` field will be recorded
in produced [`Span`]. All fields will be recorded using their `fmt::Debug`
implementation, if your field doesn't implement `fmt::Debug` you'll get compile
time error. In this case ypu should either implement `fmt::Debug` or skip it
using `#[instrument(skip(<fields to skip>))]` if you still want to record it but
for some reason you don't want to implement `fmt::Debug` trait, consider reintroducing
this field with `fields(field_name = some_value)` like shown bellow.


### Example
```rust
# extern crate juniper;
# use juniper::graphql_object;
#
# fn main() {}
#
# struct Context;
# impl juniper::Context for Context {}
# struct Query;

#[derive(Clone, juniper::GraphQLInputObject)]
struct NonDebug {
    important_field: String,
}

# #[graphql_object]
# impl Query {
// Note that you can use name of the skipped field as alias.
#[instrument(skip(non_debug), fields(non_debug = non_debug.important_field.clone()))]
fn my_query(&self, non_debug: NonDebug) -> i32 {
    // Some query
#    unimplemented!()
}
# }
```

Custom fields generated this way are context aware and can use both `context` and `self`
even if they're not implicitly passed to resolver. In case when resolver is `fn` with not
only `self` and `context` arguments they're also available to interact with as shown above.

## `#[instrument]` attribute

In most aspects it mimics behaviour of the original `#[instrument]` attribute
from [tracing] crate and you could use it as a reference. With the only key
deference you should understand, it applied implicitly to all resolvers if the
`tracing` feature is enabled.

[tracing]: https://crates.io/crates/tracing
[`skip`]: https://docs.rs/tracing/0.1.26/tracing/attr.instrument.html#skipping-fields
[`Span`]: https://docs.rs/tracing/0.1.26/tracing/struct.Span.html
