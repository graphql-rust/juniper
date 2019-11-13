# Integrating with Actix Web

[Actix web] is a small, pragmatic, and extremely fast rust web framework. For all intents and purposes it’s a microframework with a few twists. If you are already a Rust programmer you will probably find yourself at home quickly, but even if you are coming from another programming language you should find Actix web easy to pick up.

Juniper's Actix web integration is contained in the [`juniper_actix_web`][juniper_actix_web] crate:

!FILENAME Cargo.toml

```toml
[dependencies]
juniper = "0.14"
juniper_actix_web = "0.1.0"
```

Included in the source is a [small example][example] which sets up a basic GraphQL, [GraphiQL], and [GraphQL Playground] handler.

[graphiql]: https://github.com/graphql/graphiql
[graphql playground]: https://github.com/prisma-labs/graphql-playground
[actix web]: https://actix.rs/
[juniper_actix_web]: https://github.com/graphql-rust/juniper/tree/master/juniper_actix_web
[example]: https://github.com/graphql-rust/juniper/blob/master/juniper_actix_web/examples/actix_web_server.rs
