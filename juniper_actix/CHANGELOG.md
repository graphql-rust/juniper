`juniper_actix` changelog
=========================

All user visible changes to `juniper_actix` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## master

### BC Breaks

- Switched to 4.0 version of [`actix-web` crate] and its ecosystem. ([#1034])
- Switched to 0.16 version of [`juniper` crate].
- Switched to 0.4 version of [`juniper_graphql_ws` crate].
- Switched to 0.2 version of [`actix-ws` crate]. ([#1197])
- Renamed `subscriptions::subscriptions_handler()` as `subscriptions::graphql_ws_handler()` for processing the [legacy `graphql-ws` GraphQL over WebSocket Protocol][graphql-ws]. ([#1191], [#1197])

### Added

- `subscriptions::graphql_transport_ws_handler()` allowing to process the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][graphql-transport-ws]. ([#1191], [#1197])
- `subscriptions::ws_handler()` with auto-selection between the [legacy `graphql-ws` GraphQL over WebSocket Protocol][graphql-ws] and the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][graphql-transport-ws], based on the `Sec-Websocket-Protocol` HTTP header value. ([#1191], [#1197])

### Fixed

- `operationName` not being set. ([#1187], [#1169])

[#1034]: /../../pull/1034
[#1169]: /../../issues/1169
[#1187]: /../../pull/1187
[#1191]: /../../pull/1191
[#1197]: /../../pull/1197




## Previous releases

See [old CHANGELOG](/../../blob/juniper_actix-v0.4.0/juniper_actix/CHANGELOG.md).




[`actix` crate]: https://docs.rs/actix
[`actix-web` crate]: https://docs.rs/actix-web
[`actix-ws` crate]: https://docs.rs/actix-ws
[`juniper` crate]: https://docs.rs/juniper
[`juniper_graphql_ws` crate]: https://docs.rs/juniper_graphql_ws
[Semantic Versioning 2.0.0]: https://semver.org
[graphql-transport-ws]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
[graphql-ws]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md