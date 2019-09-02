# Iron 集成

> [servers/iron.md](https://github.com/graphql-rust/juniper/blob/master/docs/book/content/servers/iron.md)
> <br />
> commit 29025e6cae4a249fa56017dcf16b95ee4e89363e

[Iron] 是在 Rust 领域已存在一段时间的可靠的库，具有常见的请求（request）/响应（response）/中间件（middleware）等体系结构，运行于 Rust 稳定版。

Juniper 的 Iron 集成包为 [`juniper_iron`][juniper_iron]：

!文件名 Cargo.toml

```toml
[dependencies]
juniper = "0.10"
juniper_iron = "0.2.0"
```

[GraphiQL] 基本设定和[源码实例][example]。

[iron]: http://ironframework.io
[graphiql]: https://github.com/graphql/graphiql
[juniper_iron]: (https://github.com/graphql-rust/juniper/tree/master/juniper_iron)
[example]: https://github.com/graphql-rust/juniper/blob/master/juniper_iron/examples/iron_server.rs
