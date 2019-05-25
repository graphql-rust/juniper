# Integrating with Warp

[Warp] is a super-easy, composable, web server framework for warp speeds.
The fundamental building block of warp is the Filter: they can be combined and composed to express rich requirements on requests. Warp is built on [Hyper] and works on
Rust's stable channel.

Juniper's Warp integration is contained in the [`juniper_warp`][juniper_warp] crate:

!FILENAME Cargo.toml

```toml
[dependencies]
juniper = "0.10"
juniper_warp = "0.1.0"
```

Included in the source is a [small example][example] which sets up a basic GraphQL and [GraphiQL] handler.

[graphiql]: https://github.com/graphql/graphiql
[hyper]: https://hyper.rs/
[warp]: https://crates.io/crates/warp
[juniper_warp]: https://github.com/graphql-rust/juniper/tree/master/juniper_warp
[example]: https://github.com/graphql-rust/juniper/blob/master/juniper_warp/examples/warp_server.rs
