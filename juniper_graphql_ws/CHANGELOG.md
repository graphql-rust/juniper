`juniper_graphql_ws` changelog
==============================

All user visible changes to `juniper_graphql_ws` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## master

### BC Breaks

- `Schema::Context` now requires `Clone` bound for ability to have a "fresh" context value each time a new [GraphQL] operation is started in a [WebSocket] connection. ([#1369])
  > **COMPATIBILITY**: Previously, it was `Arc`ed inside, sharing the same context value across all [GraphQL] operations of a [WebSocket] connection. To preserve the previous behavior, the `Schema::Context` type should be either wrapped into `Arc` or made `Arc`-based internally.
- Replaced `ConnectionConfig::keep_alive_interval` option with `ConnectionConfig::keep_alive` one as `KeepAliveConfig`. ([#1367])
- Made [WebSocket] connection closed once `ConnectionConfig::keep_alive::timeout` is reached in [`graphql-transport-ws` GraphQL over WebSocket Protocol][proto-6.0.7]. ([#1367])
  > **COMPATIBILITY**: Previously, a [WebSocket] connection was kept alive, even when clients do not respond to server's `Pong` messages at all. To preserve the previous behavior, the `ConnectionConfig::keep_alive::timeout` should be set to `Duration:::ZERO`.

### Added

- `ConnectionConfig::panic_handler` field and `ConnectionConfig::with_panic_handler()` method allowing to specify `PanicHandler` for panics happened during execution of [GraphQL] operations. ([#1371])

### Changed

- Merged `graphql_transport_ws::NextPayload` and `graphql_ws::DataPayload` into a single struct. ([#1371])

### Fixed

- Inability to re-subscribe with the same operation `id` after subscription was completed by server. ([#1368])

[#1367]: /../../pull/1367
[#1368]: /../../pull/1368
[#1369]: /../../pull/1369
[#1371]: /../../pull/1371
[proto-6.0.7]: https://github.com/enisdenjo/graphql-ws/blob/v6.0.7/PROTOCOL.md




## [0.5.0] · 2025-09-08
[0.5.0]: /../../tree/juniper_graphql_ws-v0.5.0/juniper_graphql_ws

### BC Breaks

- Switched to 0.17 version of [`juniper` crate].
- Switched to 0.18 version of [`juniper_subscriptions` crate].
- Bumped up [MSRV] to 1.85. ([#1272], [1b1fc618])

[#1272]: /../../pull/1272
[1b1fc618]: /../../commit/1b1fc61879ffdd640d741e187dc20678bf7ab295




## [0.4.0] · 2024-03-20
[0.4.0]: /../../tree/juniper_graphql_ws-v0.4.0/juniper_graphql_ws

### BC Breaks

- Moved existing implementation to `graphql_ws` module implementing [legacy `graphql-ws` GraphQL over WebSocket Protocol][proto-legacy] behind `graphql-ws` Cargo feature. ([#1196])
- Switched to 0.16 version of [`juniper` crate].
- Switched to 0.17 version of [`juniper_subscriptions` crate].

### Added

- `graphql_transport_ws` module implementing [`graphql-transport-ws` GraphQL over WebSocket Protocol][proto-5.14.0] as of 5.14.0 version of [`graphql-ws` npm package] behind `graphql-transport-ws` Cargo feature. ([#1158], [#1191], [#1196], [#1197], [#1022])

### Changed

- Made fields of `ConnectionConfig` public. ([#1191])

[#1022]: /../../issues/1022
[#1158]: /../../pull/1158
[#1191]: /../../pull/1191
[#1196]: /../../pull/1196
[#1197]: /../../pull/1197
[proto-5.14.0]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
[proto-legacy]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md




## Previous releases

See [old CHANGELOG](/../../blob/juniper_graphql_ws-v0.3.0/juniper_graphql_ws/CHANGELOG.md).




[`graphql-ws` npm package]: https://npmjs.com/package/graphql-ws
[`juniper` crate]: https://docs.rs/juniper
[`juniper_subscriptions` crate]: https://docs.rs/juniper_subscriptions
[GraphQL]: https://graphql.org
[MSRV]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
[Semantic Versioning 2.0.0]: https://semver.org
[WebSocket]: https://en.wikipedia.org/wiki/WebSocket
