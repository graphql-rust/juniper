# Integrating with Rocket

[Rocket] is a web framework for Rust that makes it simple to write fast web applications without sacrificing flexibility or type safety. All with minimal code. Rocket
does not work on Rust's stable channel and instead requires the nightly
channel.

Juniper's Rocket integration is contained in the [`juniper_rocket`][juniper_rocket] crate:

!FILENAME Cargo.toml

```toml
[dependencies]
juniper = "0.10"
juniper_rocket = "0.2.0"
```

Included in the source is a [small example][example] which sets up a basic GraphQL and [GraphiQL] handler.

[graphiql]: https://github.com/graphql/graphiql
[rocket]: https://rocket.rs/
[juniper_rocket]: https://github.com/graphql-rust/juniper/tree/master/juniper_rocket
[example]: https://github.com/graphql-rust/juniper/blob/master/juniper_rocket/examples/rocket_server.rs
