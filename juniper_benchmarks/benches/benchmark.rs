//! Benchmarks for Juniper.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use juniper::{InputValue, Variables};
use std::collections::HashMap;

fn bench_users_flat_instant(c: &mut Criterion) {
    const QUERY: &'static str = r#"
        query Query($id: Int) {
            users_async_instant(ids: [$id]!) {
                id
                kind
                username
                email
            }
        }
    "#;

    let test_set = vec![1, 10];

    let mut group = c.benchmark_group("Users Flat - Instant");
    for count in test_set {
        let ids = (0..count)
            .map(|x| InputValue::scalar(x as i32))
            .collect::<Vec<_>>();
        let ids = InputValue::list(ids);
        let query_data: HashMap<_, _> =
            vec![("ids".to_string(), ids.clone())].into_iter().collect();

        group.bench_with_input(
            BenchmarkId::new("Single Thread", count),
            &query_data,
            |b, query_data| {
                let mut rt = tokio::runtime::Builder::new()
                    .basic_scheduler()
                    .build()
                    .unwrap();
                b.iter(|| {
                    let f = juniper_benchmarks::execute(QUERY, query_data.clone());
                    rt.block_on(f)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("Threadpool", count),
            &query_data,
            |b, query_data| {
                let mut rt = tokio::runtime::Builder::new()
                    .threaded_scheduler()
                    .build()
                    .unwrap();

                b.iter(|| {
                    let f = juniper_benchmarks::execute(QUERY, query_data.clone());
                    rt.block_on(f)
                })
            },
        );
    }

    group.finish();
}

fn bench_users_flat_introspection_query_type_name(c: &mut Criterion) {
    const QUERY: &'static str = r#"
        query IntrospectionQueryTypeQuery {
          __schema {
            queryType {
              name
            }
          }
        }"#;

    let mut group = c.benchmark_group("Users Flat - Introspection Query Type Name");
    group.bench_function("Single Thread", |b| {
        let mut rt = tokio::runtime::Builder::new()
            .basic_scheduler()
            .build()
            .unwrap();
        b.iter(|| {
            let f = juniper_benchmarks::execute(QUERY, Variables::new());
            rt.block_on(f)
        })
    });

    group.bench_function("Threadpool", |b| {
        let mut rt = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .build()
            .unwrap();

        b.iter(|| {
            let f = juniper_benchmarks::execute(QUERY, Variables::new());
            rt.block_on(f)
        })
    });

    group.finish();
}

fn bench_users_flat_introspection_query_full(c: &mut Criterion) {
    const QUERY: &'static str = r#"
  query IntrospectionQuery {
    __schema {
      queryType { name }
      mutationType { name }
      subscriptionType { name }
      types {
        ...FullType
      }
      directives {
        name
        description
        locations
        args {
          ...InputValue
        }
      }
    }
  }

  fragment FullType on __Type {
    kind
    name
    description
    fields(includeDeprecated: true) {
      name
      description
      args {
        ...InputValue
      }
      type {
        ...TypeRef
      }
      isDeprecated
      deprecationReason
    }
    inputFields {
      ...InputValue
    }
    interfaces {
      ...TypeRef
    }
    enumValues(includeDeprecated: true) {
      name
      description
      isDeprecated
      deprecationReason
    }
    possibleTypes {
      ...TypeRef
    }
  }

  fragment InputValue on __InputValue {
    name
    description
    type { ...TypeRef }
    defaultValue
  }

  fragment TypeRef on __Type {
    kind
    name
    ofType {
      kind
      name
      ofType {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
              ofType {
                kind
                name
                ofType {
                  kind
                  name
                }
              }
            }
          }
        }
      }
    }
  }
"#;

    let mut group = c.benchmark_group("Users Flat - Introspection Query Full");
    group.bench_function("Single Thread", |b| {
        let mut rt = tokio::runtime::Builder::new()
            .basic_scheduler()
            .build()
            .unwrap();
        b.iter(|| {
            let f = juniper_benchmarks::execute(QUERY, Variables::new());
            rt.block_on(f)
        })
    });

    group.bench_function("Threadpool", |b| {
        let mut rt = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .build()
            .unwrap();

        b.iter(|| {
            let f = juniper_benchmarks::execute(QUERY, Variables::new());
            rt.block_on(f)
        })
    });

    group.finish();
}

fn bench_users_flat_introspection_query_without_description(c: &mut Criterion) {
    const QUERY: &'static str = r#"
query IntrospectionQuery {
  __schema {
    queryType {
      name
    }
    mutationType {
      name
    }
    subscriptionType {
      name
    }
    types {
      ...FullType
    }
    directives {
      name
      locations
      args {
        ...InputValue
      }
    }
  }
}
fragment FullType on __Type {
  kind
  name
  fields(includeDeprecated: true) {
    name
    args {
      ...InputValue
    }
    type {
      ...TypeRef
    }
    isDeprecated
    deprecationReason
  }
  inputFields {
    ...InputValue
  }
  interfaces {
    ...TypeRef
  }
  enumValues(includeDeprecated: true) {
    name
    isDeprecated
    deprecationReason
  }
  possibleTypes {
    ...TypeRef
  }
}
fragment InputValue on __InputValue {
  name
  type {
    ...TypeRef
  }
  defaultValue
}
fragment TypeRef on __Type {
  kind
  name
  ofType {
    kind
    name
    ofType {
      kind
      name
      ofType {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
              ofType {
                kind
                name
              }
            }
          }
        }
      }
    }
  }
}
"#;

    let mut group = c.benchmark_group("Users Flat - Introspection Query Without Descprtion");
    group.bench_function("Single Thread", |b| {
        let mut rt = tokio::runtime::Builder::new()
            .basic_scheduler()
            .build()
            .unwrap();
        b.iter(|| {
            let f = juniper_benchmarks::execute(QUERY, Variables::new());
            rt.block_on(f)
        })
    });

    group.bench_function("Threadpool", |b| {
        let mut rt = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .build()
            .unwrap();

        b.iter(|| {
            let f = juniper_benchmarks::execute(QUERY, Variables::new());
            rt.block_on(f)
        })
    });

    group.finish();
}

criterion_group!(
    users_flat,
    bench_users_flat_instant,
    bench_users_flat_introspection_query_type_name,
    bench_users_flat_introspection_query_full,
    bench_users_flat_introspection_query_without_description,
);
criterion_main!(users_flat);
