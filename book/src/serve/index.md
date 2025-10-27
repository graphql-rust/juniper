Serving
=======

Once we have built a [GraphQL schema][1], the next obvious step would be to serve it, so clients can interact with our [GraphQL] API. Usually, [GraphQL] APIs are served via [HTTP]. 




## Web server frameworks

Though the [`juniper`] crate doesn't provide a built-in [HTTP] server, the surrounding ecosystem does.


### Officially supported

[Juniper] officially supports the following widely used and adopted web server frameworks in [Rust] ecosystem:
- [`actix-web`] ([`juniper_actix`] crate)
- [`axum`] ([`juniper_axum`] crate)
- [`hyper`] ([`juniper_hyper`] crate)
- [`rocket`] ([`juniper_rocket`] crate)
- [`warp`] ([`juniper_warp`] crate)

See their API docs and usage examples (accessible from API docs) for further details of how they should be used.

> **NOTE**: All the officially supported web server framework integrations provide a simple and convenient way for exposing [GraphiQL] and/or [GraphQL Playground] with the [GraphQL schema][1] along. These powerful tools ease the development process by enabling you to explore and send client requests to the [GraphQL] API under development.




## WebSocket

> **NOTE**: [WebSocket] is a crucial part for serving [GraphQL subscriptions][2] over [HTTP].

There are two widely adopted protocols for serving [GraphQL] over [WebSocket]:
1. [Legacy `graphql-ws` GraphQL over WebSocket Protocol][ws-old], formerly used by [Apollo] and the [`subscriptions-transport-ws` npm package], and now being deprecated.
2. [New `graphql-transport-ws` GraphQL over WebSocket Protocol][ws-new], provided by the [`graphql-ws` npm package] and being used by [Apollo] as for now.

In the [Juniper] ecosystem, both implementations are provided by the [`juniper_graphql_ws`] crate. Most of the [officially supported web server framework integrations](#officially-supported) are able to serve a [GraphQL schema][1] over [WebSocket] (including [subscriptions][2]) and even support [auto-negotiation of the correct protocol based on the `Sec-Websocket-Protocol` HTTP header value][3]. See their API docs and usage examples (accessible from API docs) for further details of how to do so.




[`actix-web`]: https://docs.rs/actix-web
[`axum`]: https://docs.rs/axum
[`graphql-ws` npm package]: https://npmjs.com/package/graphql-ws
[`juniper`]: https://docs.rs/juniper
[`juniper_actix`]: https://docs.rs/juniper_actix
[`juniper_axum`]: https://docs.rs/juniper_axum
[`juniper_graphql_ws`]: https://docs.rs/juniper_graphql_ws
[`juniper_rocket`]: https://docs.rs/juniper_rocket
[`juniper_warp`]: https://docs.rs/juniper_warp
[`hyper`]: https://docs.rs/hyper
[`rocket`]: https://docs.rs/rocket
[`subscriptions-transport-ws` npm package]: https://npmjs.com/package/subscriptions-transport-ws
[`warp`]: https://docs.rs/warp
[Apollo]: https://www.apollographql.com
§
[GraphQL]: https://graphql.org
[GraphQL Playground]: https://github.com/prisma/graphql-playground
[HTTP]: https://en.wikipedia.org/wiki/HTTP
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org
[WebSocket]: https://en.wikipedia.org/wiki/WebSocket
[ws-new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
[ws-old]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md

[1]: ../schema/index.md
[2]: ../schema/subscriptions.md
[3]: https://developer.mozilla.org/docs/Web/API/WebSockets_API/Writing_WebSocket_servers#subprotocols