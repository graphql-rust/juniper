extern crate juniper_benchmarks;

use criterion::{black_box, criterion_group, criterion_main, Criterion, ParameterizedBenchmark};

use juniper::{graphql_value, InputValue, ToInputValue, Value};
use juniper_benchmarks as j;

fn bench_sync_vs_async_single_user_flat_instant(c: &mut Criterion) {
    const QUERY: &'static str = r#"
        query Query($id: Int) {
            user(id: $id) {
                id
                kind
                username
                email
            }
        }
    "#;

    c.bench(
        "Sync vs Async - Single User Flat - Instant",
        ParameterizedBenchmark::new(
            "Sync",
            |b, count| {
                let ids = (0..*count)
                    .map(|x| InputValue::scalar(x as i32))
                    .collect::<Vec<_>>();
                let ids = InputValue::list(ids);
                b.iter(|| {
                    j::execute(
                        QUERY,
                        vec![("ids".to_string(), ids.clone())].into_iter().collect(),
                    )
                })
            },
            vec![1, 10],
        )
        .with_function("Async - Single Thread", |b, count| {
            let mut rt = tokio::runtime::current_thread::Runtime::new().unwrap();

            let ids = (0..*count)
                .map(|x| InputValue::scalar(x as i32))
                .collect::<Vec<_>>();
            let ids = InputValue::list(ids);

            b.iter(|| {
                let f = j::execute_async(
                    QUERY,
                    vec![("ids".to_string(), ids.clone())].into_iter().collect(),
                );
                rt.block_on(f)
            })
        })
        .with_function("Async - Threadpool", |b, count| {
            let rt = tokio::runtime::Runtime::new().unwrap();

            let ids = (0..*count)
                .map(|x| InputValue::scalar(x as i32))
                .collect::<Vec<_>>();
            let ids = InputValue::list(ids);

            b.iter(|| {
                let f = j::execute_async(
                    QUERY,
                    vec![("ids".to_string(), ids.clone())].into_iter().collect(),
                );
                rt.block_on(f)
            })
        }),
    );
}

fn bench_sync_vs_async_introspection_query_instant(c: &mut Criterion) {
    c.bench(
        "Sync vs Async - Introspection Query - Instant",
        ParameterizedBenchmark::new("Sync", |b, _| b.iter(|| j::introspect()), vec![1, 10])
            .with_function("Async - Single Thread", |b, _| {
                let mut rt = tokio::runtime::current_thread::Runtime::new().unwrap();
                b.iter(|| {
                    let f = j::introspect_async();
                    rt.block_on(f)
                })
            })
            .with_function("Async - Threadpool", |b, _| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                b.iter(|| {
                    let f = j::introspect_async();
                    rt.block_on(f)
                })
            }),
    );
}

criterion_group!(
    query_benches,
    bench_sync_vs_async_single_user_flat_instant,
    bench_sync_vs_async_introspection_query_instant,
);
criterion_main!(query_benches);
