`juniper_warp` changelog
========================

All user visible changes to `juniper_warp` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## master

### BC Breaks

- Switched to 0.16 version of [`juniper` crate].

### Added

- `subscriptions::serve_graphql_transport_ws()` function allowing to process the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][graphql-transport-ws]. ([#1158])
- `subscriptions::make_ws_filter()` function providing endpoint with auto-selection between the [legacy `graphql-ws` GraphQL over WebSocket Protocol][graphql-ws] and the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][graphql-transport-ws], based on the `Sec-Websocket-Protocol` HTTP header value. ([#1191])

### Changed

- Made `schema` argument of `make_graphql_filter()` and `make_graphql_filter_sync()` polymorphic, allowing to specify external `Arc`ed `schema`. ([#1136], [#1135])

[#1135]: /../../issues/1136
[#1136]: /../../pull/1136
[#1158]: /../../pull/1158
[#1191]: /../../pull/1191




## Previous releases

See [old CHANGELOG](/../../blob/juniper_warp-v0.7.0/juniper_warp/CHANGELOG.md).




[`juniper` crate]: https://docs.rs/juniper
[Semantic Versioning 2.0.0]: https://semver.org
[graphql-transport-ws]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
[graphql-ws]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md
