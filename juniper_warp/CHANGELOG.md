`juniper_warp` changelog
========================

All user visible changes to `juniper_warp` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## master

### BC Breaks

- Bumped up [MSRV] to 1.75. ([#1272])

[#1272]: /../../pull/1272




## [0.8.0] · 2024-03-20
[0.8.0]: /../../tree/juniper_warp-v0.8.0/juniper_warp

### BC Breaks

- Switched to 0.16 version of [`juniper` crate].
- Removed `JoinError` from public API. ([#1222], [#1177])

### Added

- `subscriptions::serve_graphql_transport_ws()` function allowing to process the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][graphql-transport-ws]. ([#1158])
- `subscriptions::make_ws_filter()` function providing endpoint with auto-selection between the [legacy `graphql-ws` GraphQL over WebSocket Protocol][graphql-ws] and the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][graphql-transport-ws], based on the `Sec-Websocket-Protocol` HTTP header value. ([#1191])

### Changed

- Made `schema` argument of `make_graphql_filter()` and `make_graphql_filter_sync()` polymorphic, allowing to specify external `Arc`ed `schema`. ([#1136], [#1135])
- Relaxed requirement for `context_extractor` to be a `BoxedFilter` only. ([#1222], [#1177])

### Fixed

- Excessive `context_extractor` execution in `make_graphql_filter()` and `make_graphql_filter_sync()`. ([#1222], [#1177])

[#1135]: /../../issues/1136
[#1136]: /../../pull/1136
[#1158]: /../../pull/1158
[#1177]: /../../issues/1177
[#1191]: /../../pull/1191
[#1222]: /../../pull/1222




## Previous releases

See [old CHANGELOG](/../../blob/juniper_warp-v0.7.0/juniper_warp/CHANGELOG.md).




[`juniper` crate]: https://docs.rs/juniper
[graphql-transport-ws]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
[graphql-ws]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md
[MSRV]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
[Semantic Versioning 2.0.0]: https://semver.org
