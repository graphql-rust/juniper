`juniper_subscriptions` crate
=============================

[![Crates.io](https://img.shields.io/crates/v/juniper_subscriptions.svg?maxAge=2592000)](https://crates.io/crates/juniper_subscriptions)
[![Documentation](https://docs.rs/juniper_subscriptions/badge.svg)](https://docs.rs/juniper_subscriptions)
[![CI](https://github.com/graphql-rust/juniper/workflows/CI/badge.svg?branch=master "CI")](https://github.com/graphql-rust/juniper/actions?query=workflow%3ACI+branch%3Amaster)
[![Rust 1.65+](https://img.shields.io/badge/rustc-1.65+-lightgray.svg "Rust 1.65+")](https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html)

- [Changelog](https://github.com/graphql-rust/juniper/blob/master/juniper_subscriptions/CHANGELOG.md)

This repository contains `SubscriptionCoordinator` and `SubscriptionConnection` implementations for 
[`juniper`], a [GraphQL] library for Rust.

You need both this and [`juniper`] crate for usage.




## Documentation

For this crate's documentation, check out [API docs](https://docs.rs/juniper_subscriptions).

For `SubscriptionCoordinator` and `SubscriptionConnection` documentation, check out [`juniper` API docs][`juniper`]. 




## Examples

Check [`juniper_warp/examples/subscription.rs`][1] for example code of a working [`warp`] server with [GraphQL] subscription handlers.




## License

This project is licensed under [BSD 2-Clause License](https://github.com/graphql-rust/juniper/blob/master/juniper_subscriptions/LICENSE).




[`juniper`]: https://docs.rs/juniper
[`warp`]: https://docs.rs/warp
[GraphQL]: http://graphql.org

[1]: https://github.com/graphql-rust/juniper/blob/master/juniper_warp/examples/subscription.rs
