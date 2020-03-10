extern crate juniper_benchmarks;

use criterion::{black_box, criterion_group, criterion_main, Criterion, ParameterizedBenchmark};

use juniper::{graphql_value, InputValue, ToInputValue, Value};
use juniper_benchmarks as j;

fn bench_sync_vs_async_users_flat_instant(c: &mut Criterion) {
    const ASYNC_QUERY: &'static str = r#"
        query Query($id: Int) {
            users_async_instant(ids: [$id]!) {
                id
                kind
                username
                email
            }
        }
    "#;

    const SYNC_QUERY: &'static str = r#"
    query Query($id: Int) {
        users_sync_instant(ids: [$id]!) {
            id
            kind
            username
            email
        }
    }
"#;

    c.bench(
        "Sync vs Async - Users Flat - Instant",
        ParameterizedBenchmark::new(
            "Sync",
            |b, count| {
                let ids = (0..*count)
                    .map(|x| InputValue::scalar(x as i32))
                    .collect::<Vec<_>>();
                let ids = InputValue::list(ids);
                b.iter(|| {
                    j::execute_sync(
                        SYNC_QUERY,
                        vec![("ids".to_string(), ids.clone())].into_iter().collect(),
                    )
                })
            },
            vec![1, 10],
        )
        .with_function("Async - Single Thread", |b, count| {
            let mut rt = tokio::runtime::Builder::new()
                .basic_scheduler()
                .build()
                .unwrap();

            let ids = (0..*count)
                .map(|x| InputValue::scalar(x as i32))
                .collect::<Vec<_>>();
            let ids = InputValue::list(ids);

            b.iter(|| {
                let f = j::execute(
                    ASYNC_QUERY,
                    vec![("ids".to_string(), ids.clone())].into_iter().collect(),
                );
                rt.block_on(f)
            })
        })
        .with_function("Async - Threadpool", |b, count| {
            let mut rt = tokio::runtime::Builder::new()
                .threaded_scheduler()
                .build()
                .unwrap();

            let ids = (0..*count)
                .map(|x| InputValue::scalar(x as i32))
                .collect::<Vec<_>>();
            let ids = InputValue::list(ids);

            b.iter(|| {
                let f = j::execute(
                    ASYNC_QUERY,
                    vec![("ids".to_string(), ids.clone())].into_iter().collect(),
                );
                rt.block_on(f)
            })
        }),
    );
}

criterion_group!(benches, bench_sync_vs_async_users_flat_instant);
criterion_main!(benches);
