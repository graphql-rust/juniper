# Tracing

Starting from version `0.15.8` Juniper has optional support for the [tracing] crate for instrumentation.

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
    
    // Multiplying is much harder, so we'll trace it, recorded span will also
    // contain value of `another`.
    fn multiply_values(&self, another: i32) -> i32 {
        self.value * another
    }
    
    // Squaring is also hard, and for the scientific needs we're interested in
    // the value that was squared so we should record it. In this case we can
    // use `fields` argument to pass additional fields, that also will be
    // included in span.
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

# `#[instrument]` attribute

Juniper has it's own `#[instrument]` attribute, that can only be used with
GraphQL Objects and Interfaces. In most aspects it mimics behavior of the original
`#[instrument]` attribute from [tracing] crate including fields, sigils, so you
could use it as a reference. First key deference you should keep in mind: `#[instrument]`
applied implicitly to **all** resolvers if the `tracing` feature is enabled.
Second and most significant difference is generated [`Span`]s. To fully understand
this you should know how GraphQL Objects/Interfaces are actually resolved under
the hood, long story short there is a lot of generated code and two methods
`resolve_field` and `resolve_field_async` that map your resolvers to fields,
and then recursively resolve fields of returned value (if it's not a `Scalar`).
`#[instrument]` from [tracing] knows nothing about Juniper and all dark magic
performed, so it will only wrap method in [`Span`], ignoring recursive part,
effectively capturing only tip of the iceberg, so this will lead to plain sequence
of resolver [`Span`]s, where you could hardly understand order and relations between
each  resolver. On the other hand `#[instrument]` from Juniper is part of top-level
macro so it's aware of all tricks performed by Juniper and will be expanded as part
of `resolve_field` or `resolve_field_async`, [`Span`]s generated this way will capture
full lifespan of value, including how it's fields resolved which results in more tree-like
[`Span`] structure in which you could easily navigate through, using tools like
[Jaeger]. As a bonus you'll get [`Span`] names, which refer to your schema instead
of code.

## Skipping field resolvers

In certain scenarios you may want to skip tracing of some fields because it's too
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
    // This won't produce span because it's marked with `tracing(ignore)`
    #[graphql(tracing(ignore))]
    fn id(&self) -> i32 {
        self.id
    }

    // This will still produce span
    async fn friends(&self, context: &Context) -> Vec<User> {
        // Some async query in which you're actually interested.
#       unimplemented!()
    }
}
```

Manually setting `#[graphql(tracing(ignore))]` to skip tracing of all, let's
say for example, synchronous field resolvers is rather inefficient when you have 
GraphQL object with too much fields. In this case you can use `tracing` attribute
on top-level to trace specific group of fields or not to trace at all.
`tracing` attribute can be used with one of the following arguments:
`sync`, `async`, `only` or `skip_all`.
 - Use `sync` to trace only synchronous resolvers (struct fields and `fn`s).
 - Use `async` to trace only asynchronous resolvers (`async fn`s) and
subscriptions.
 - Use `only` to trace only fields marked with `#[graphql(tracing(only))]`.
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
time error. In this case you should either implement `fmt::Debug` or skip it
using `#[instrument(skip(<fields to skip>))]` if you still want to record it but
for some reason you don't want to (or cannot) implement `fmt::Debug` trait, consider
reintroducing this field with `fields(field_name = some_value)` like shown bellow.


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
    important_field: i32,
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
to interact with as shown above. You can also access `executor` and a `Context` as a result.

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

## `Empty` fields

Field names may also be specified without values. Doing so will result in an
empty field whose value may be recorded later within the function body. This
can be done by adding field with value `Empty` (see [`field::Empty`]) and 
accessing `tracing::Span::current()` within resolver body.

### Example

```rust
# extern crate juniper;
# use juniper::graphql_object;
#
# fn main() {}
#
use tracing::field; // alternatively you can use `juniper::tracing::field`

#[derive(Debug)]
struct Foo;

struct Query;

# #[graphql_object]
# impl Query {
#[instrument(fields(later = field::Empty))]
async fn empty_field() -> i32 {
    // Use `record("<field_name>", &value)` to record value into empty field.
    tracing::Span::current().record("later", &"see ya later!");
    // resolver code
#   unimplemented!()
}

#[instrument(fields(msg = "Everything is OK."))]
async fn override_field() -> i32 {
    // We can override `msg` with the same syntax as we recorded `later` in
    // example above. In fact `Empty` field is a special case of overriding.
    tracing::Span::current().record("msg", &"Everything is Perfect!");
    // In cases when you want to record a non-standard value to span you may
    // use `field::debug(...)` and `field::display(...)`, to set proper formatting.
    tracing::Span::current().record("msg", &field::debug(&Foo));
    // Doing `tracing::Span::current().record("msg", Foo)` will result in
    // compilation error.
#   unimplemented!()
}
# }
```

## Error handling

When resolver returns `Result<T, E>`, you can add `err` argument to `#[instrument]`
attribute, doing so you will create empty field `err` that will be recorded if your
resolver function will return `Result::Err(...)`. If it used with any type other than
`Result<T, E>` it will result in compilation error.

Additionally you could do this manually using the `Empty` field and manual recording.
This is more versatile solution and as a tradeoff, it applies additional responsibility
on user code so should be used with care.

### Example

```rust
# extern crate juniper;
# use std::fmt;
# use juniper::{graphql_object, FieldError};
#
# fn main() {}
#
# struct Query;
use tracing::field;

#[derive(Debug)]
struct Foo;

struct MyError;

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Definitely not an error, trust me")
    }
}

impl<S> juniper::IntoFieldError<S> for MyError {
    fn into_field_error(self) -> FieldError<S> {
        FieldError::new(self, juniper::Value::Null)
    }
}

# #[graphql_object]
# impl Query {
// Under the hood it will create `Empty` field with name "err", and will record
// it within `resolve_field` or `resolve_field_async`.
#[instrument(err)]
async fn my_calculation() -> Result<i32, MyError> {
    Err(MyError)
}

// Here we have manually created `Empty` field with name "err".
#[instrument(fields(err = field::Empty))]
async fn conditional_error() -> Result<i32, MyError> {
    let res = Err(MyError);
#   let condition = false;
    // At this point we manually check whether result is error or not and if
    // condition is met and only then record error.
    if condition {
        if let Err(err) = &res  {
            tracing::Span::current().record("err", &field::display(err));
        }
    } 
    res
}
# }
```

## Subscriptions and tracing

Subscriptions a little bit harder to trace than futures and other resolvers,
they can (and in most cases will) produce sequences of results and resolving
will produce multiple almost identical groups of spans, to address this issue
Juniper has two layers of [`Span`]s, first a global one, represents whole
subscription from start and until last result (or first Error). And local
second layer, it represents **field resolution process** of a single value
returned by subscription. It won't capture how object is queried, but stream
**may** do this under some conditions.

For example you have subscription that queries database once a second for each
subscriber, in this case you can easily trace every individual step. But once
we introduce more users following the best practices we should perform this
queries in some sort of a batch. Juniper offers coordinators which perform some
magic under the hood to manage all subscriptions. For example scheduling multiple
subscription to single database query. This raises a question, which span should
be picked as parent for this query, and answer is implementation dependent, so
should be handled manually. 

[tracing]: https://crates.io/crates/tracing
[`skip`]: https://docs.rs/tracing/0.1.26/tracing/attr.instrument.html#skipping-fields
[`Span`]: https://docs.rs/tracing/0.1.26/tracing/struct.Span.html
[`field::Empty`]: https://docs.rs/tracing/0.1.26/tracing/field/struct.Empty.html
[Jaeger]: https://www.jaegertracing.io
