#![allow(missing_docs)]

use std::collections::HashMap;

use futures::stream::{self, BoxStream};
use tracing::instrument;

use crate::{graphql_interface, graphql_object, graphql_subscription, Context, GraphQLObject};

#[derive(Debug)]
pub struct Database {
    inner: HashMap<i32, String>,
}

impl Database {
    pub fn new() -> Self {
        let mut inner = HashMap::new();
        inner.insert(42, "Meaning of life".to_owned());
        Self { inner }
    }

    /// Query mock, instrumented by `tracing` crate.
    #[instrument(skip(self))]
    pub fn traced_query(&self, id: i32) -> Option<String> {
        self.inner.get(&id).cloned()
    }

    /// Non traced query mock.
    pub fn non_traced_query(&self, id: i32) -> Option<String> {
        self.inner.get(&id).cloned()
    }
}

impl Context for Database {}

/// Query root with various queries used to test tracing compatibility.
pub struct Query;

#[graphql_object(context = Database)]
impl Query {
    /// Simple sync query with no arguments.
    fn foo() -> Foo {
        Foo { id: 37 }
    }

    /// Sync query with argument.
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

    /// Async query with argument.
    async fn async_bar(id: i32) -> Bar {
        Bar { id }
    }

    /// Query that returns `Foo` wrapped in `FooBar`.
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

    /// Returns GraphQL object marked with `trace = "async"`.
    async fn trace_async() -> TraceAsync {
        TraceAsync
    }

    /// Returns derived GraphQL object marked with `trace = "async"`.
    async fn derived_async() -> AsyncDerived {
        AsyncDerived::default()
    }

    /// Returns GraphQL object marked with `trace = "sync"`.
    fn trace_sync() -> TraceSync {
        TraceSync
    }

    /// Returns derived GraphQL object marked with `trace = "sync"`.
    fn derived_sync() -> SyncDerived {
        SyncDerived::default()
    }

    /// Returns GraphQL object marked with `trace = "skip-all"`.
    fn skip_all() -> SkipAll {
        SkipAll
    }

    /// Returns derived GraphQL object marked with `trace = "skip-all"`.
    fn skip_all_derived() -> SkipAllDerived {
        SkipAllDerived::default()
    }

    /// Returns GraphQL object marked with `trace = "complex"` in sync manner.
    fn complex_sync() -> Complex {
        Complex
    }

    /// Returns GraphQL object marked with `trace = "complex"` in async manner.
    async fn complex_async() -> Complex {
        Complex
    }

    /// Returns derived GraphQL object marked with `trace = "complex"`.
    fn complex_derived() -> DerivedComplex {
        DerivedComplex {
            complex: false,
            another_complex: false,
            sync: 0,
        }
    }

    /// Returns GraphQL object wrapped in `InterfacedSimple` GraphQL interface.
    fn erased_simple() -> InterfacedSimpleValue {
        InterfacedSimpleValue::TraceSync(TraceSync)
    }

    /// Returns GraphQL object wrapped in GraphQL interface marked with `trace = "sync"`.
    fn erased_sync() -> InterfacedSyncValue {
        InterfacedSyncValue::TraceSync(TraceSync)
    }

    /// Returns GraphQL object wrapped in GraphQL interface marked with `trace = "async"`.
    fn erased_async() -> InterfacedAsyncValue {
        InterfacedAsyncValue::TraceAsync(TraceAsync)
    }

    /// Returns GraphQL object wrapped in GraphQL interface marked with `trace = "skip-all"`.
    fn erased_skip_all() -> InterfacedSkipAllValue {
        InterfacedSkipAllValue::SkipAll(SkipAll)
    }

    /// Returns GraphQL object wrapped in GraphQL interface marked with `trace = "complex"`.
    fn erased_complex() -> InterfacedComplexValue {
        InterfacedComplexValue::Complex(Complex)
    }
}

pub struct Subscriptions;

#[graphql_subscription(context = Database)]
impl Subscriptions {
    async fn bar_sub(id: i32) -> BoxStream<'static, Bar> {
        let items = [Bar { id: id + 1 }, Bar { id: id + 2 }];

        stream::iter(items).boxed()
    }
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

    /// Sync field marked with `no_trace`.
    #[tracing(no_trace)]
    fn non_traced(&self) -> &str {
        "None can trace this"
    }

    /// Async field marked with `no_trace`.
    #[tracing(no_trace)]
    async fn async_non_traced(&self) -> &str {
        "None can trace this"
    }

    /// Field with multiple arguments, one of which is skipped.
    #[tracing(skip(name))]
    fn skip_argument(&self, name: String, meaning_of_life: i32) -> i32 {
        let _ = name;
        meaning_of_life
    }

    /// Field with it's `target` overwritten.
    #[tracing(target = "my_target")]
    fn target(&self) -> bool {
        true
    }

    /// Field with it's `level` overwritten.
    #[tracing(level = "warn")]
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
    #[tracing(fields(self.id = self.id))]
    fn id(&self) -> i32 {
        self.id
    }

    /// Field that has signature identical to `FooBar`s one but in fact traced.
    fn non_traced(&self) -> bool {
        false
    }

    /// Field with default arguments.
    #[graphql(arguments(this(default = 42), another(default = 0), skipped(default = 1),))]
    #[tracing(skip(skipped))]
    fn default_arg(&self, this: i32, another: i32, skipped: i32) -> i32 {
        this + another + skipped
    }

    /// Traced database query.
    async fn traced_data(&self, context: &Database) -> Option<String> {
        context.traced_query(self.id)
    }

    /// Non traced database query.
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

/// Derived `GraphQLObject`.
#[derive(GraphQLObject)]
#[graphql(impl = FooBarValue, context = Database)]
pub struct DerivedFoo {
    /// Resolver from that has context bound and const bound trace fields.
    #[tracing(fields(self.id = self.id, custom_fields = "work"))]
    id: i32,

    /// Field marked with `no_trace` within derived GraphQLObject.
    #[tracing(no_trace)]
    non_traced: String,

    /// Field with it's `target` overwritten.
    #[tracing(target = "my_target")]
    target: bool,

    /// Field with it's `level` overwritten.
    #[tracing(level = "warn")]
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

    /// Interface field marked with `no_trace`.
    #[tracing(no_trace)]
    fn non_traced(&self) -> bool {
        true
    }

    /// Async interface field marked with `no_trace`.
    #[tracing(no_trace)]
    async fn async_non_traced(&self) -> bool {
        true
    }

    /// Interface field with various arguments.
    #[tracing(skip(skipped))]
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
    #[tracing(skip(skipped))]
    async fn async_with_arg(
        &self,
        id: i32,
        skipped: i32,
        #[graphql(default = 0)] default: i32,
        #[graphql(default = 1)] overwritten: i32,
    ) -> i32 {
        id + skipped + default + overwritten
    }

    #[tracing(target = "my_target")]
    fn target(&self) -> i32 {
        1
    }

    #[tracing(level = "warn")]
    fn level(&self) -> i32 {
        2
    }
}

/// GraphQL object marked with `trace = "skip-sync"`
pub struct TraceSync;

#[graphql_object(
    trace = "sync",
    impl = [InterfacedSimpleValue, InterfacedSyncValue],
)]
impl TraceSync {
    pub fn sync_fn(&self) -> i32 {
        1
    }

    pub async fn async_fn(&self) -> i32 {
        2
    }
}

/// Derived GraphQL object marked with `trace = "sync"`.
#[derive(Default, GraphQLObject)]
#[graphql(trace = "sync")]
pub struct SyncDerived {
    /// Simple field
    sync: i32,
}

/// GraphQL object marked with `trace = "async"`.
pub struct TraceAsync;

#[graphql_object(
    trace = "async",
    impl = [InterfacedAsyncValue],
)]
impl TraceAsync {
    pub fn sync_fn(&self) -> i32 {
        1
    }

    pub async fn async_fn(&self) -> i32 {
        2
    }
}

/// Derived GraphQL object
#[derive(Default, GraphQLObject)]
#[graphql(trace = "async")]
pub struct AsyncDerived {
    /// Simple field
    sync: i32,
}

/// GraphQL object marked with `trace = "skip-all"`.
pub struct SkipAll;

#[graphql_object(
    trace = "skip-all",
    impl = [InterfacedSkipAllValue],
)]
impl SkipAll {
    pub fn sync_fn(&self) -> i32 {
        1
    }

    pub async fn async_fn(&self) -> i32 {
        2
    }
}

#[derive(Default, GraphQLObject)]
#[graphql(trace = "skip-all")]
pub struct SkipAllDerived {
    /// Simple field
    sync: i32,
}

/// GraphQL object marked with `trace = "complex"`
pub struct Complex;

#[graphql_object(
    trace = "complex",
    impl = [InterfacedComplexValue],
)]
impl Complex {
    #[tracing(complex)]
    pub fn sync_fn(&self) -> i32 {
        1
    }

    #[tracing(complex)]
    pub async fn async_fn(&self) -> i32 {
        2
    }

    fn simple_field(&self) -> i32 {
        3
    }

    #[tracing(complex, no_trace)]
    fn no_trace_complex(&self) -> i32 {
        4
    }
}

#[derive(GraphQLObject)]
#[graphql(trace = "complex")]
pub struct DerivedComplex {
    #[tracing(complex)]
    complex: bool,
    #[tracing(complex, fields(test = "magic"))]
    another_complex: bool,

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

#[graphql_interface(
    for = [TraceSync],
    trace = "sync",
    async,
)]
trait InterfacedSync {
    fn sync_fn(&self) -> i32;
    async fn async_fn(&self) -> i32;
}

#[graphql_interface(
    for = [TraceAsync],
    trace = "async",
    async,
)]
trait InterfacedAsync {
    fn sync_fn(&self) -> i32;
    async fn async_fn(&self) -> i32;
}

#[graphql_interface(
    for = [SkipAll],
    trace = "skip-all",
    async,
)]
trait InterfacedSkipAll {
    fn sync_fn(&self) -> i32;
    async fn async_fn(&self) -> i32;
}

#[graphql_interface(
    for = [Complex],
    trace = "complex",
    async,
)]
trait InterfacedComplex {
    fn sync_fn(&self) -> i32;
    #[tracing(complex)]
    async fn async_fn(&self) -> i32;
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

build_impl!(TraceSync, InterfacedSimple);
build_impl!(TraceSync, InterfacedSync);
build_impl!(TraceAsync, InterfacedAsync);
build_impl!(SkipAll, InterfacedSkipAll);
build_impl!(Complex, InterfacedComplex);