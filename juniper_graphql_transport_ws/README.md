`juniper_graphql_transport_ws` crate
====================================

[![Crates.io](https://img.shields.io/crates/v/juniper_graphql_transport_ws.svg?maxAge=2592000)](https://crates.io/crates/juniper_graphql_transport_ws)
[![Documentation](https://docs.rs/juniper_graphql_transport_ws/badge.svg)](https://docs.rs/juniper_graphql_transport_ws)
[![CI](https://github.com/graphql-rust/juniper/workflows/CI/badge.svg?branch=master "CI")](https://github.com/graphql-rust/juniper/actions?query=workflow%3ACI+branch%3Amaster)
[![Rust 1.65+](https://img.shields.io/badge/rustc-1.65+-lightgray.svg "Rust 1.65+")](https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html)

- [Changelog](https://github.com/graphql-rust/juniper/blob/master/juniper_graphql_transport_ws/CHANGELOG.md)

This crate contains an implementation of the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new], as now used by [Apollo] and [`graphql-ws` npm package].

Implementation of the [legacy `graphql-ws` GraphQL over WebSocket Protocol][old] may be found in the [`juniper_graphql_ws` crate].




## License

This project is licensed under [BSD 2-Clause License](https://github.com/graphql-rust/juniper/blob/master/juniper_graphql_transport_ws/LICENSE).




[`graphql-ws` npm package]: https://npmjs.com/package/graphql-ws
[`juniper_graphql_ws` crate]: https://docs.rs/juniper_graphql_ws
[Apollo]: https://www.apollographql.com
[new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
[old]: https://github.com/apollographql/subscriptions-transport-ws/blob/0ce7a1e1eb687fe51214483e4735f50a2f2d5c79/PROTOCOL.md
