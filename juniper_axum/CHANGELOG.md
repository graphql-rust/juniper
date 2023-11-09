`juniper_axum` changelog
========================

All user visible changes to `juniper_axum` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## master

### Initialized

- Dependent on 0.6 version of [`axum` crate]. ([#1088])
- Dependent on 0.16 version of [`juniper` crate]. ([#1088])
- Dependent on 0.4 version of [`juniper_graphql_ws` crate]. ([#1088])

### Added

- `extract::JuniperRequest` and `response::JuniperResponse` for using in custom [`axum` crate] handlers. ([#1088])
- `graphql` handler processing [GraphQL] requests for the specified schema. ([#1088], [#1184])
- `subscriptions::graphql_transport_ws()` handler and `subscriptions::serve_graphql_transport_ws()` function allowing to process the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][graphql-transport-ws]. ([#1088], [#986])
- `subscriptions::graphql_ws()` handler and `subscriptions::serve_graphql_ws()` function allowing to process the [legacy `graphql-ws` GraphQL over WebSocket Protocol][graphql-ws]. ([#1088], [#986])
- `subscriptions::ws()` handler and `subscriptions::serve_ws()` function allowing to auto-select between the [legacy `graphql-ws` GraphQL over WebSocket Protocol][graphql-ws] and the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][graphql-transport-ws], based on the `Sec-Websocket-Protocol` HTTP header value. ([#1088], [#986])
- `graphiql` handler serving [GraphiQL]. ([#1088])
- `playground` handler serving [GraphQL Playground]. ([#1088])
- `simple.rs` and `custom.rs` integration examples. ([#1088], [#986], [#1184])

[#986]: /../../issues/986
[#1088]: /../../pull/1088
[#1184]: /../../issues/1184




[`axum` crate]: https://docs.rs/axum
[`juniper` crate]: https://docs.rs/juniper
[`juniper_graphql_ws` crate]: https://docs.rs/juniper_graphql_ws
[GraphiQL]: https://github.com/graphql/graphiql
[GraphQL]: http://graphql.org
[GraphQL Playground]: https://github.com/prisma/graphql-playground
[Semantic Versioning 2.0.0]: https://semver.org
[graphql-transport-ws]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
[graphql-ws]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md