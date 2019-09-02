# Hyper 集成

> [servers/hyper.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/servers/hyper.md)
> <br />
> commit 9623e4d32694118e68ce8706f29e2cfbc6c5b6dc

[Hyper] 是影响了许多 Rust Web 框架的敏捷 HTTP 实现。它通过 Tokio 运行时提供异步 I/O，并在 Rust 稳定版上工作。

Hyper 并非高层次 Web 框架，因此不包含一些通用特性，诸如：简单的终端路由、后端内建 HTTP 响应，以及可重用中间件等。对于 GraphQL 来说，这些并不是很大的不足，因为所有的 POSTs、GETs 通常都经过一个端点，并有一些响应明确定义的有效负载。

Juniper 的 Hyper 集成包为 [`juniper_hyper`][juniper_hyper]：

!文件名 Cargo.toml

```toml
[dependencies]
juniper = "0.10"
juniper_hyper = "0.1.0"
```

[GraphiQL] 基本设定和[源码实例][example]。

[graphiql]: https://github.com/graphql/graphiql
[hyper]: https://hyper.rs/
[juniper_hyper]: https://github.com/graphql-rust/juniper/tree/master/juniper_hyper
[example]: https://github.com/graphql-rust/juniper/blob/master/juniper_hyper/examples/hyper_server.rs
