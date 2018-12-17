# master

- No changes yet

# [0.2.0] 2018-12-17

- **[Breaking Change]** The minimum required `warp` version is now `0.1.8`.

  [#271](https://github.com/graphql-rust/juniper/pull/271)

- **[Breaking Change]** The `graphql_handler` and `graphiql_handler` functions have been renamed to`graphql_filter` and `graphiql_filter` respectively.

  [#267](https://github.com/graphql-rust/juniper/pull/267)

- **[Breaking Change]** A `CpuPool` from `futures_cpupool` is no longer used. Instead, `warp`'s underlying `tokio_threadpool` is leveraged. Because of this, `make_graphql_filter_with_thread_pool` is no longer necessary and has been removed.

  [#258](https://github.com/graphql-rust/juniper/pull/258)

# juniper_warp [0.1] 2018-09-13

- Initial Release
