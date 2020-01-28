# Integrating with Hyper

[Hyper] is a fast HTTP implementation that many other Rust web frameworks
leverage. It offers asynchronous I/O via the tokio runtime and works on
Rust's stable channel.

Hyper is not a higher-level web framework and accordingly
does not include ergonomic features such as simple endpoint routing,
baked-in HTTP responses, or reusable middleware. For GraphQL, those aren't
large downsides as all POSTs and GETs usually go through a single endpoint with
a few clearly-defined response payloads.

Juniper's Hyper integration is contained in the [`juniper_hyper`][juniper_hyper] crate:

!FILENAME Cargo.toml

```toml
[dependencies]
juniper = "0.10"
juniper_hyper = "0.1.0"
```

Included in the source is a [small example][example] which sets up a basic GraphQL and [GraphiQL] handler.

[graphiql]: https://github.com/graphql/graphiql
[hyper]: https://hyper.rs/
[juniper_hyper]: https://github.com/graphql-rust/juniper/tree/master/juniper_hyper
[example]: https://github.com/graphql-rust/juniper/blob/master/juniper_hyper/examples/hyper_server.rs
