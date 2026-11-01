//! Compares the engines on every operation in [`operation::ALL`].

#![allow(
    unused_crate_dependencies,
    reason = "the library uses the other dependencies"
)]

use criterion::{
    BenchmarkGroup, Criterion, criterion_group, criterion_main, measurement::WallTime,
};
use juniper_benchmarks::{
    engine::{
        Engine,
        apollo_compiler::ApolloCompiler,
        async_graphql::AsyncGraphql,
        juniper::{JuniperAsync, JuniperAsyncFn, JuniperSync},
    },
    operation::{self, Operation},
};

fn compare(c: &mut Criterion) {
    for operation in operation::ALL {
        operation.check();
        let request = operation.request();
        let mut group = c.benchmark_group(operation.name);
        bench::<JuniperAsync>(&mut group, operation, &request);
        bench::<JuniperSync>(&mut group, operation, &request);
        bench::<JuniperAsyncFn>(&mut group, operation, &request);
        bench::<AsyncGraphql>(&mut group, operation, &request);
        bench::<ApolloCompiler>(&mut group, operation, &request);
        group.finish();
    }
}

/// Measures the `E` engine on the `operation`, if the engine supports the operation.
fn bench<E: Engine>(group: &mut BenchmarkGroup<'_, WallTime>, operation: Operation, request: &str) {
    if !operation.runs_on::<E>() {
        return;
    }
    let engine = E::default();
    group.bench_function(E::NAME, |b| b.iter(|| engine.execute(request)));
}

criterion_group!(benches, compare);
criterion_main!(benches);
