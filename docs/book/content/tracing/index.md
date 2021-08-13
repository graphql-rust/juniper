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
object with too much fields. In this case you can use `tracing` attribute on
top-level to skip tracing of specific field group or not to trace at all.
`tracing` attribute can be used with one of the following arguments:
`sync`, `async`, `only` or `skip_all`.
 - Use `sync` to trace only synchronous resolvers (struct fields and `fn`s).
 - Use `async` to trace only asynchronous resolvers (`async fn`s) and
subscriptions.
 - Use `only` to trace only fields marked with `#[graphql(tracing(only))]`
 - Use `skip_all` to skip tracing of all fields.

### Example

```rust
# extern crate juniper;
# use juniper::graphql_object;
# fn main() {}

struct MagicOfTracing;

#[graphql_object]
#[tracing(async)]
impl MagicOfTracing {
    // Won't produce span because it's sync resolver
    fn my_sync_fn(&self) -> String {
        "Woah sync resolver!!".to_owned()
    }

    // Will produce span `MagicOfTracing.myAsyncFn`.
    async fn my_async_fn(&self) -> String {
        "Woah async resolver with traces!!".to_owned()
    }

    // Won't produce span because even though this is an async resolver
    // it's also marked with `#[graphql(tracing(ignore))]`.
    #[graphql(tracing(ignore))]
    async fn non_traced_async_fn(&self) -> String {
        "Leave no traces".to_owned()
    }
}
```

**Note:** using of `tracing(sync)` with derived struct is no-op because all
resolvers within derived GraphQL object is considered to be synchronous, also
because of this `tracing(async)` will result in no traces.

In addition you can use `#[graphql(tracing(ignore))]` with `sync` and `async`
variants to exclude field from tracing even if it belongs to traced group.

**Be careful when skipping trace as it can lead to bad structured span trees,
disabling of tracing on one level won't disable tracing in it's child methods.
As a rule of thumb you should trace all field resolvers which may produce child
spans.**

If resolving of certain field requires additional arguments (when used `fn`s or
`async fn`s) they also will be included in resulted trace (except `self` and
`Context`).

```rust
# extern crate juniper;
# use juniper::{graphql_object, GraphQLObject};
#
# fn main() {}
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

Custom fields generated this way are aware of `self` and can use `self` even if it not implicitly passed
to resolver. In case when resolver is `fn` with not only `self` arguments they're also available
to interact with as shown above. You can also access `executor` and `Context` as a result.

### Example
```rust
# extern crate juniper;
# use juniper::graphql_object;
#
# fn main() {}
#
struct Context {
    data: i32,
}

impl juniper::Context for Context {}

struct Query {
    data: i32,
}

#[graphql_object(context = Context)]
impl Query {
#[instrument(fields(ctx.data = executor.context().data))]
fn my_query(&self) -> i32 {
    // Some query
#    unimplemented!()
}

#[instrument(fields(data = self.data))]
fn self_aware() -> i32 {
    // Some query
#   unimplemented!()
}
# }
```

## `#[instrument]` attribute

In most aspects it mimics behavior of the original `#[instrument]` attribute
from [tracing] crate and you could use it as a reference. With the only key
deference you should understand, it applied implicitly to all resolvers if the
`tracing` feature is enabled.

[tracing]: https://crates.io/crates/tracing
[`skip`]: https://docs.rs/tracing/0.1.26/tracing/attr.instrument.html#skipping-fields
[`Span`]: https://docs.rs/tracing/0.1.26/tracing/struct.Span.html
