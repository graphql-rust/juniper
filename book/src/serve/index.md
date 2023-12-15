Serving
=======

Once we have built a [GraphQL schema][1], the next obvious step would be to serve it, so clients can interact with our [GraphQL] API. Usually, [GraphQL] APIs are served via [HTTP]. 




## Web server frameworks

Though, [`juniper`] crate itself doesn't provide a built-in [HTTP] server, the surrounding ecosystem does.


### Officially supported

[Juniper] officially supports the following widely used and adopted web server frameworks in [Rust] ecosystem:
- [`actix-web`] ([`juniper_actix`] crate)
- [`axum`] ([`juniper_axum`] crate)
- [`hyper`] ([`juniper_hyper`] crate)
- [`rocket`] ([`juniper_rocket`] crate)
- [`warp`] ([`juniper_warp`] crate)

See their API docs and usage examples (accessible from API docs) for further details of how they should be used.

> **NOTE**: All the officially supported web server framework integrations also provide a simple and convenient way for exposing [GraphiQL] and/or [GraphQL Playground] along with the [GraphQL schema][1]. These powerful tools may ease the development process drastically, as allow to explore the created [GraphQL] API and to interact with it from the client side.




[`actix-web`]: https://docs.rs/actix-web
[`axum`]: https://docs.rs/axum
[`juniper`]: https://docs.rs/juniper
[`juniper_actix`]: https://docs.rs/juniper_actix
[`juniper_axum`]: https://docs.rs/juniper_axum
[`juniper_hyper`]: https://docs.rs/juniper_hyper
[`juniper_rocket`]: https://docs.rs/juniper_rocket
[`juniper_warp`]: https://docs.rs/juniper_warp
[`hyper`]: https://docs.rs/hyper
[`rocket`]: https://docs.rs/rocket
[`warp`]: https://docs.rs/warp
[GraphiQL]: https://github.com/graphql/graphiql
[GraphQL]: https://graphql.org
[GraphQL Playground]: https://github.com/prisma/graphql-playground
[HTTP]: https://en.wikipedia.org/wiki/HTTP
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org

[1]: ../schema/schemas_and_mutations.md