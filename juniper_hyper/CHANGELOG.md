# master

- Compatibility with the latest `juniper`.

# [[0.4.0] 2019-07-19](https://github.com/graphql-rust/juniper/releases/tag/juniper_hyper-0.4.0)

- Compatibility with the latest `juniper`.

# [[0.3.0] 2019-05-16](https://github.com/graphql-rust/juniper/releases/tag/juniper_hyper-0.3.0)

- Compatibility with the latest `juniper`.

# 0.2.0 [2018-12-18]

## Breaking changes

- The tokio threadpool managed by `hyper` is now used for
  executing GraphQL operations as well. Previously a separate threadpool from `futures_cpupool` was required.

  [#256](https://github.com/graphql-rust/juniper/pull/256)
