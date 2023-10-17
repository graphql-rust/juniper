`juniper_graphql_ws` crate
==========================

[![Crates.io](https://img.shields.io/crates/v/juniper_graphql_ws.svg?maxAge=2592000)](https://crates.io/crates/juniper_graphql_ws)
[![Documentation](https://docs.rs/juniper_graphql_ws/badge.svg)](https://docs.rs/juniper_graphql_ws)
[![CI](https://github.com/graphql-rust/juniper/workflows/CI/badge.svg?branch=master "CI")](https://github.com/graphql-rust/juniper/actions?query=workflow%3ACI+branch%3Amaster)
[![Rust 1.65+](https://img.shields.io/badge/rustc-1.65+-lightgray.svg "Rust 1.65+")](https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html)

- [Changelog](https://github.com/graphql-rust/juniper/blob/master/juniper_graphql_ws/CHANGELOG.md)

This crate contains implementations of 2 protocols:

1. (`graphql-transport-ws` feature) The [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new], as now used by [Apollo] and [`graphql-ws` npm package].

2. (`graphql-ws` feature) The [legacy `graphql-ws` GraphQL over WebSocket Protocol][old], as formerly used by [Apollo] and [`subscriptions-transport-ws` npm package] (deprecated in favor of the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new] mentioned above).




## License

This project is licensed under [BSD 2-Clause License](https://github.com/graphql-rust/juniper/blob/master/juniper_graphql_ws/LICENSE).




[`graphql-ws` npm package]: https://npmjs.com/package/graphql-ws
[`subscriptions-transport-ws` npm package]: https://npmjs.com/package/subscriptions-transport-ws
[Apollo]: https://www.apollographql.com
[new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
[old]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md
