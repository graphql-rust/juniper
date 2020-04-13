# master

- Compatibility with the latest `juniper`.
- Changed the implementation place of GraphQLBatchRequest and GraphQLBatchResponse in `juniper_warp`
to `juniper` to be reused in other http integrations, since this implementation was private.

## Breaking Changes

- Update `playground_filter` to support subscription endpoint URLs
- Update `warp` to 0.2
- Rename synchronous `execute` to `execute_sync`, add asynchronous `execute`
- `juniper_warp::graphiql_filter` now requires a second parameter for subscriptions

# [[0.5.2] 2019-12-16](https://github.com/graphql-rust/juniper/releases/tag/juniper_warp-0.5.2)

- Compatibility with the latest `juniper`.

# [[0.5.1] 2019-10-24](https://github.com/graphql-rust/juniper/releases/tag/juniper_warp-0.5.1)

- Compatibility with the latest `juniper`.

# [[0.5.0] 2019-09-29](https://github.com/graphql-rust/juniper/releases/tag/juniper_warp-0.5.0)

- Compatibility with the latest `juniper`.

# [[0.4.1] 2019-07-29](https://github.com/graphql-rust/juniper/releases/tag/juniper_warp-0.4.1)

- Compatibility with the latest `juniper`.

# [[0.4.0] 2019-07-19](https://github.com/graphql-rust/juniper/releases/tag/juniper_warp-0.4.0)

- Compatibility with the latest `juniper`.

# [[0.3.0] 2019-05-16](https://github.com/graphql-rust/juniper/releases/tag/juniper_warp-0.3.0)

- Compatibility with the latest `juniper`.

# [0.2.0] 2018-12-17

- **[Breaking Change]** The minimum required `warp` version is now `0.1.8`.

  [#271](https://github.com/graphql-rust/juniper/pull/271)

- **[Breaking Change]** The `graphql_handler` and `graphiql_handler` functions have been renamed to`graphql_filter` and `graphiql_filter` respectively.

  [#267](https://github.com/graphql-rust/juniper/pull/267)

- **[Breaking Change]** A `CpuPool` from `futures_cpupool` is no longer used. Instead, `warp`'s underlying `tokio_threadpool` is leveraged. Because of this, `make_graphql_filter_with_thread_pool` is no longer necessary and has been removed.

  [#258](https://github.com/graphql-rust/juniper/pull/258)

# juniper_warp [0.1] 2018-09-13

- Initial Release
