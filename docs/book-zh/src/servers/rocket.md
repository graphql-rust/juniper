# Rocket 集成

> [servers/rocket.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/servers/rocket.md)
> <br />
> commit 9623e4d32694118e68ce8706f29e2cfbc6c5b6dc

[Rocket] 是使 Web 应用开发变得简单高效和类型安全的 Rust Web 框架。Rocket 运行于 Rust 开发版，不能在 Rust 稳定版上工作。

Juniper 的 Rocket 集成包为 [`juniper_rocket`][juniper_rocket]：

!文件名 Cargo.toml

```toml
[dependencies]
juniper = "0.10"
juniper_rocket = "0.2.0"
```

[GraphiQL] 基本设定和[源码实例][example]。

[graphiql]: https://github.com/graphql/graphiql
[rocket]: https://rocket.rs/
[juniper_rocket]: https://github.com/graphql-rust/juniper/tree/master/juniper_rocket
[example]: https://github.com/graphql-rust/juniper/blob/master/juniper_rocket/examples/rocket_server.rs
