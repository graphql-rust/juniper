use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use juniper::InputValue;
use juniper_benchmarks as j;

fn bench_sync_vs_async_users_flat_instant(c: &mut Criterion) {
    // language=GraphQL
    const ASYNC_QUERY: &str = r#"
        query Query($id: Int) {
            users_async_instant(ids: [$id]!) {
                id
                kind
                username
                email
            }
        }
    "#;

    // language=GraphQL
    const SYNC_QUERY: &str = r#"
        query Query($id: Int) {
            users_sync_instant(ids: [$id]!) {
                id
                kind
                username
                email
            }
        }
    "#;

    let mut group = c.benchmark_group("Sync vs Async - Users Flat - Instant");
    for count in [1, 10] {
        group.bench_function(BenchmarkId::new("Sync", count), |b| {
            let ids = (0..count).map(InputValue::scalar).collect::<Vec<_>>();
            let ids = InputValue::list(ids);
            b.iter(|| {
                j::execute_sync(
                    SYNC_QUERY,
                    vec![("ids".to_owned(), ids.clone())].into_iter().collect(),
                )
            })
        });

        group.bench_function(BenchmarkId::new("Async - Single Thread", count), |b| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();

            let ids = (0..count).map(InputValue::scalar).collect::<Vec<_>>();
            let ids = InputValue::list(ids);

            b.iter(|| {
                let f = j::execute(
                    ASYNC_QUERY,
                    vec![("ids".to_owned(), ids.clone())].into_iter().collect(),
                );
                rt.block_on(f)
            })
        });

        group.bench_function(BenchmarkId::new("Async - Threadpool", count), |b| {
            let rt = tokio::runtime::Builder::new_multi_thread().build().unwrap();

            let ids = (0..count).map(InputValue::scalar).collect::<Vec<_>>();
            let ids = InputValue::list(ids);

            b.iter(|| {
                let f = j::execute(
                    ASYNC_QUERY,
                    vec![("ids".to_owned(), ids.clone())].into_iter().collect(),
                );
                rt.block_on(f)
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_sync_vs_async_users_flat_instant);
criterion_main!(benches);
