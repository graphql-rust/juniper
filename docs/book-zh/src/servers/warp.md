# Warp 集成

> [servers/warp.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/servers/warp.md)
> <br />
> commit c2f119690b683303dbabf2a8b029cff76b596728

[Warp] 是超简单、可组合、具有极快速度的 Web 服务器框架。Warp 基本构建块是过滤器：过滤器可组合表示需求多样的请求。Warp 构建在 [Hyper] 之上，运行于 Rust 稳定版。

Juniper 的 Warp 集成包为 [`juniper_warp`][juniper_warp]：

!文件名 Cargo.toml

```toml
[dependencies]
juniper = "0.10"
juniper_warp = "0.1.0"
```

[GraphiQL] 基本设定和[源码实例][example]。

[graphiql]: https://github.com/graphql/graphiql
[hyper]: https://hyper.rs/
[warp]: https://crates.io/crates/warp
[juniper_warp]: https://github.com/graphql-rust/juniper/tree/master/juniper_warp
[example]: https://github.com/graphql-rust/juniper/blob/master/juniper_warp/examples/warp_server.rs
