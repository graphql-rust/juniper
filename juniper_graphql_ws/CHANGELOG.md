`juniper_graphql_ws` changelog
==============================

All user visible changes to `juniper_graphql_ws` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## master

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
[Semantic Versioning 2.0.0]: https://semver.org
