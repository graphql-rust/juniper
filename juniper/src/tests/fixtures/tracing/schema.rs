//! Schema that contains all the necessities to test integration with
//! [`tracing`] crate.

use std::{collections::HashMap, fmt};

use futures::stream::{self, BoxStream, StreamExt as _};

use crate::{
    graphql_interface, graphql_object, graphql_subscription,
    tracing::{self, field, instrument},
    Context, FieldError, GraphQLObject,
};

/// Test database.
#[derive(Debug)]
pub struct Database {
    inner: HashMap<i32, String>,
}

impl Context for Database {}

impl Database {
    /// Returns a new [`Database`].
    pub fn new() -> Self {
        let mut inner = HashMap::new();
        inner.insert(42, "Meaning of life".to_owned());
        Self { inner }
    }

    /// Query mock, instrumented by [`tracing`] crate.
    #[instrument(skip(self))]
    pub fn traced_query(&self, id: i32) -> Option<String> {
        self.inner.get(&id).cloned()
    }

    /// Non traced query mock.
    pub fn non_traced_query(&self, id: i32) -> Option<String> {
        self.inner.get(&id).cloned()
    }

    /// Sync query mock, instrumented by [`tracing`] crate.
    #[instrument(skip(self))]
    pub async fn async_traced_query(&self, id: i32) -> Option<String> {
        self.inner.get(&id).cloned()
    }
}

/// Query root with various queries used to test [`tracing`] compatibility.
pub struct Query;

#[graphql_object(context = Database)]
impl Query {
    /// Simple sync query with no arguments.
    fn foo() -> Foo {
        Foo { id: 37 }
    }

    /// Sync query with an argument.
    fn bar(id: i32) -> Bar {
        Bar { id }
    }

    /// Simple async query.
    async fn async_foo() -> DerivedFoo {
        DerivedFoo {
            id: 42,
            non_traced: "None can trace this".to_owned(),
            target: false,
            level: false,
        }
    }

    /// Async query with an argument.
    async fn async_bar(id: i32) -> Bar {
        Bar { id }
    }

    /// Query that returns an object wrapped into GraphQL interface.
    fn foo_bar() -> FooBarValue {
        FooBarValue::Foo(Foo { id: 1 })
    }

    /// Query that returns collection of objects wrapped into GraphQL interface.
    fn foo_bars(id: i32) -> Vec<FooBarValue> {
        vec![
            FooBarValue::Foo(Foo { id }),
            FooBarValue::Bar(Bar { id }),
            FooBarValue::DerivedFoo(DerivedFoo {
                id,
                non_traced: "leave no traces".to_owned(),
                target: false,
                level: false,
            }),
        ]
    }

    /// Returns GraphQL object marked with `tracing(async)`.
    async fn trace_async() -> TraceAsync {
        TraceAsync
    }

    /// Returns derived GraphQL object marked with `tracing(async)`.
    async fn derived_async() -> AsyncDerived {
        AsyncDerived::default()
    }

    /// Returns GraphQL object marked with `tracing(sync)`.
    fn trace_sync() -> TraceSync {
        TraceSync
    }

    /// Returns derived GraphQL object marked with `tracing(sync)`.
    fn derived_sync() -> SyncDerived {
        SyncDerived::default()
    }

    /// Returns GraphQL object marked with `tracing(skip_all)`.
    fn skip_all() -> SkipAll {
        SkipAll
    }

    /// Returns derived GraphQL object marked with `tracing(skip_all)`.
    fn skip_all_derived() -> SkipAllDerived {
        SkipAllDerived::default()
    }

    /// Returns GraphQL object marked with `tracing(only)` in sync manner.
    fn only_sync() -> Only {
        Only
    }

    /// Returns GraphQL object marked with `tracing(only)` in async manner.
    async fn only_async() -> Only {
        Only
    }

    /// Returns derived GraphQL object marked with `tracing(only)`.
    fn only_derived() -> DerivedOnly {
        DerivedOnly {
            only: false,
            another_only: false,
            sync: 0,
        }
    }

    /// Returns GraphQL object wrapped in `InterfacedSimple` GraphQL interface.
    fn erased_simple() -> InterfacedSimpleValue {
        InterfacedSimpleValue::TraceSync(TraceSync)
    }

    /// Returns GraphQL object wrapped in GraphQL interface marked with `tracing(sync)`.
    fn erased_sync() -> InterfacedSyncValue {
        InterfacedSyncValue::TraceSync(TraceSync)
    }

    /// Returns GraphQL object wrapped in GraphQL interface marked with `tracing(async)`.
    fn erased_async() -> InterfacedAsyncValue {
        InterfacedAsyncValue::TraceAsync(TraceAsync)
    }

    /// Returns GraphQL object wrapped in GraphQL interface marked with `tracing(skip_all)`.
    fn erased_skip_all() -> InterfacedSkipAllValue {
        InterfacedSkipAllValue::SkipAll(SkipAll)
    }

    /// Returns GraphQL object wrapped in GraphQL interface marked with `tracing(only)`.
    fn erased_only() -> InterfacedOnlyValue {
        InterfacedOnlyValue::Only(Only)
    }

    /// Sync fn that uses instrumented function under the hood.
    fn sub_resolver(context: &Database) -> Option<String> {
        context.traced_query(42)
    }

    /// Async fn that uses instrumented function under the hood.
    async fn sub_async_resolver(context: &Database) -> Option<String> {
        context.async_traced_query(42).await
    }

    /// Fn that has custom field marked with debug sigil (`?`).
    #[instrument(fields(sigil = ?Sigil))]
    fn debug_sigil() -> i32 {
        1
    }

    /// Fn that has custom field marked with display sigil (`%`).
    #[instrument(fields(sigil = %Sigil))]
    fn display_sigil() -> i32 {
        1
    }

    /// Fn that has custom field that's can be recorded later.
    #[instrument(fields(magic = field::Empty))]
    async fn empty_field() -> i32 {
        tracing::Span::current().record("magic", &"really magic");
        1
    }

    /// Async fn that will record it's error.
    #[instrument(err)]
    async fn record_err_async(should_err: bool) -> Result<i32, Error> {
        if should_err {
            Err(Error)
        } else {
            Ok(1)
        }
    }

    /// Async fn that will record it's error.
    #[instrument(err)]
    fn record_err_sync(should_err: bool) -> Result<i32, Error> {
        if should_err {
            Err(Error)
        } else {
            Ok(1)
        }
    }
}

/// Custom error used to test `#[instrument(err)]` functionality.
pub struct Error;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Definitely not an error, trust me")
    }
}

impl<S> juniper::IntoFieldError<S> for Error {
    fn into_field_error(self) -> FieldError<S> {
        FieldError::new(self, juniper::Value::Null)
    }
}

/// Subscriptions root with various queries used to test [`tracing`] compatibility.
pub struct Subscriptions;

#[graphql_subscription(context = Database)]
impl Subscriptions {
    /// Subscription that emits `Bar`s.
    async fn bar_sub(id: i32) -> BoxStream<'static, Bar> {
        let items = [Bar { id: id + 1 }, Bar { id: id + 2 }];

        stream::iter(items).boxed()
    }

    /// Subscription that emits `Result<T, E>`.
    #[instrument(err)]
    async fn err_sub() -> BoxStream<'static, Result<i32, Error>> {
        let items = [Err(Error)];

        stream::iter(items).boxed()
    }
}

macro_rules! build_impl {
    ($ty:ident, $trt:ident) => {
        #[graphql_interface(async)]
        impl $trt for $ty {
            fn sync_fn(&self) -> i32 {
                1
            }

            async fn async_fn(&self) -> i32 {
                2
            }
        }
    };
}

/// Simple GraphQL object.
pub struct Foo {
    id: i32,
}

#[graphql_object(context = Database, impl = FooBarValue)]
impl Foo {
    /// Sync field calculated from `self`.
    fn id(&self) -> i32 {
        self.id
    }

    /// Sync field marked with `tracing(ignore)`.
    #[graphql(tracing(ignore))]
    fn non_traced(&self) -> &str {
        "None can trace this"
    }

    /// Async field marked with `tracing(ignore)`.
    #[graphql(tracing(ignore))]
    async fn async_non_traced(&self) -> &str {
        "None can trace this"
    }

    /// Field with multiple arguments, one of which is skipped.
    #[instrument(skip(name))]
    fn skip_argument(&self, name: String, meaning_of_life: i32) -> i32 {
        let _ = name;
        meaning_of_life
    }

    /// Field with its `target` being overwritten.
    #[instrument(target = "my_target")]
    fn target(&self) -> bool {
        true
    }

    /// Field with its `level` being overwritten.
    #[instrument(level = "warn")]
    fn level(&self) -> bool {
        true
    }
}

#[graphql_interface(async)]
impl FooBar for Foo {
    fn is_foo(&self) -> bool {
        true
    }

    async fn is_bar(&self) -> bool {
        false
    }
}

/// Simple GraphQL object with more advanced field resolvers.
pub struct Bar {
    id: i32,
}

#[graphql_object(context = Database, impl = FooBarValue)]
impl Bar {
    /// Custom field.
    #[instrument(fields(self.id = self.id))]
    fn id(&self) -> i32 {
        self.id
    }

    /// Field having signature identical to `FooBar`'s one, but being traced in fact.
    fn non_traced(&self) -> bool {
        false
    }

    /// Field with default arguments.
    #[instrument(skip(skipped))]
    fn default_arg(
        &self,
        #[graphql(default = 42)] this: i32,
        #[graphql(default = 0)] another: i32,
        #[graphql(default = 1)] skipped: i32,
    ) -> i32 {
        this + another + skipped
    }

    /// Traced database query.
    async fn traced_data(&self, context: &Database) -> Option<String> {
        context.traced_query(self.id)
    }

    /// Non-traced database query.
    async fn non_traced_data(&self, context: &Database) -> Option<String> {
        context.non_traced_query(self.id)
    }
}

#[graphql_interface(async)]
impl FooBar for Bar {
    fn is_foo(&self) -> bool {
        false
    }

    async fn is_bar(&self) -> bool {
        true
    }
}

/// Derived [`GraphQLObject`].
#[derive(GraphQLObject)]
#[graphql(impl = FooBarValue, context = Database)]
pub struct DerivedFoo {
    /// Resolver having context bound and const bound trace fields.
    #[instrument(fields(self.id = self.id, custom_fields = "work"))]
    id: i32,

    /// Field marked with `tracing(ignore)` within derived [`GraphQLObject`].
    #[graphql(tracing(ignore))]
    non_traced: String,

    /// Field with its `target` being overwritten.
    #[instrument(target = "my_target")]
    target: bool,

    /// Field with its `level` being overwritten.
    #[instrument(level = "warn")]
    level: bool,
}

#[graphql_interface(async)]
impl FooBar for DerivedFoo {
    fn is_foo(&self) -> bool {
        true
    }

    async fn is_bar(&self) -> bool {
        false
    }
}

/// GraphQL interface with various tracing features.
#[graphql_interface(for = [DerivedFoo, Foo, Bar], context = Database, async)]
pub trait FooBar {
    /// Simple sync field.
    fn is_foo(&self) -> bool;

    /// Simple async field.
    async fn is_bar(&self) -> bool;

    /// Interface field marked with `tracing(ignore)`.
    #[graphql(tracing(ignore))]
    fn non_traced(&self) -> bool {
        true
    }

    /// Async interface field marked with `tracing(ignore)`.
    #[graphql(tracing(ignore))]
    async fn async_non_traced(&self) -> bool {
        true
    }

    /// Interface field with various arguments.
    #[instrument(skip(skipped))]
    fn with_arg(
        &self,
        id: i32,
        skipped: i32,
        #[graphql(default = 0)] default: i32,
        #[graphql(default = 1)] overwritten: i32,
    ) -> i32 {
        id + skipped + default + overwritten
    }

    /// Async interface field with various arguments.
    #[instrument(skip(skipped))]
    async fn async_with_arg(
        &self,
        id: i32,
        skipped: i32,
        #[graphql(default = 0)] default: i32,
        #[graphql(default = 1)] overwritten: i32,
    ) -> i32 {
        id + skipped + default + overwritten
    }

    /// Field with its `target` being overwritten.
    #[instrument(target = "my_target")]
    fn target(&self) -> i32 {
        1
    }

    /// Field with its `level` being overwritten.
    #[instrument(level = "warn")]
    fn level(&self) -> i32 {
        2
    }
}

/// GraphQL object marked with `tracing(sync)`.
pub struct TraceSync;

#[graphql_object(impl = [InterfacedSimpleValue, InterfacedSyncValue])]
#[tracing(sync)]
impl TraceSync {
    pub fn sync_fn(&self) -> i32 {
        1
    }

    pub async fn async_fn(&self) -> i32 {
        2
    }
}

build_impl!(TraceSync, InterfacedSimple);
build_impl!(TraceSync, InterfacedSync);

/// Derived GraphQL object marked with `tracing(sync)`.
#[derive(Default, GraphQLObject)]
#[tracing(sync)]
pub struct SyncDerived {
    /// Simple field
    sync: i32,
}

/// GraphQL object marked with `tracing(async)`.
pub struct TraceAsync;

#[graphql_object(impl = [InterfacedAsyncValue])]
#[tracing(async)]
impl TraceAsync {
    pub fn sync_fn(&self) -> i32 {
        1
    }

    pub async fn async_fn(&self) -> i32 {
        2
    }
}

build_impl!(TraceAsync, InterfacedAsync);

/// Derived GraphQL object.
#[derive(Default, GraphQLObject)]
#[tracing(async)]
pub struct AsyncDerived {
    /// Simple field
    sync: i32,
}

/// GraphQL object marked with `tracing(skip_all)`.
pub struct SkipAll;

#[graphql_object(impl = [InterfacedSkipAllValue])]
#[tracing(skip_all)]
impl SkipAll {
    pub fn sync_fn(&self) -> i32 {
        1
    }

    pub async fn async_fn(&self) -> i32 {
        2
    }
}

build_impl!(SkipAll, InterfacedSkipAll);

/// Derived GraphQL object marked with `tracing(skip_all)`.
#[derive(Default, GraphQLObject)]
#[tracing(skip_all)]
pub struct SkipAllDerived {
    /// Simple field
    sync: i32,
}

/// GraphQL object marked with `tracing(only)`.
pub struct Only;

#[graphql_object(impl = [InterfacedOnlyValue])]
#[tracing(only)]
impl Only {
    #[graphql(tracing(only))]
    pub fn sync_fn(&self) -> i32 {
        1
    }

    #[graphql(tracing(only))]
    pub async fn async_fn(&self) -> i32 {
        2
    }

    fn simple_field(&self) -> i32 {
        3
    }
}

build_impl!(Only, InterfacedOnly);

/// Derived GraphQL object marked with `tracing(only)`.
#[derive(GraphQLObject)]
#[tracing(only)]
pub struct DerivedOnly {
    #[graphql(tracing(only))]
    only: bool,
    #[graphql(tracing(only))]
    #[instrument(fields(test = "magic"))]
    another_only: bool,

    /// Simple field
    sync: i32,
}

#[graphql_interface(
    for = [TraceSync],
    async,
)]
trait InterfacedSimple {
    fn sync_fn(&self) -> i32;
    async fn async_fn(&self) -> i32;
}

#[graphql_interface(for = [TraceSync], async)]
#[tracing(sync)]
trait InterfacedSync {
    fn sync_fn(&self) -> i32;
    async fn async_fn(&self) -> i32;
}

#[graphql_interface(for = [TraceAsync], async)]
#[tracing(async)]
trait InterfacedAsync {
    fn sync_fn(&self) -> i32;
    async fn async_fn(&self) -> i32;
}

#[graphql_interface(for = [SkipAll], async)]
#[tracing(skip_all)]
trait InterfacedSkipAll {
    fn sync_fn(&self) -> i32;
    async fn async_fn(&self) -> i32;
}

#[graphql_interface(for = [Only], async)]
#[tracing(only)]
trait InterfacedOnly {
    fn sync_fn(&self) -> i32;
    #[graphql(tracing(only))]
    async fn async_fn(&self) -> i32;
}

/// Value that used to test debug and display sigils.
struct Sigil;

impl fmt::Debug for Sigil {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("Debug Sigil")
    }
}

impl fmt::Display for Sigil {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("Display Sigil")
    }
}
