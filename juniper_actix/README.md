`juniper_actix` crate
=====================

[![Crates.io](https://img.shields.io/crates/v/juniper_actix.svg?maxAge=2592000)](https://crates.io/crates/juniper_actix)
[![Documentation](https://docs.rs/juniper_actix/badge.svg)](https://docs.rs/juniper_actix)
[![CI](https://github.com/graphql-rust/juniper/workflows/CI/badge.svg?branch=master "CI")](https://github.com/graphql-rust/juniper/actions?query=workflow%3ACI+branch%3Amaster)
[![Rust 1.68+](https://img.shields.io/badge/rustc-1.68+-lightgray.svg "Rust 1.68+")](https://blog.rust-lang.org/2023/03/09/Rust-1.68.0.html)

- [Changelog](https://github.com/graphql-rust/juniper/blob/master/juniper_actix/CHANGELOG.md)

[`actix-web`] web server integration for [`juniper`] ([GraphQL] implementation for [Rust]).

It's inspired and some parts are copied from [`juniper_warp`] crate.




## Documentation

For documentation, including guides and examples, check out [Juniper Book].

A basic usage example can also be found in the [API docs][`juniper_actix`].




## Examples

Check [`examples/actix_server.rs`][1] for example code of a working [`actix-web`] server with [GraphQL] handlers.




## License

This project is licensed under [BSD 2-Clause License](https://github.com/graphql-rust/juniper/blob/master/juniper_actix/LICENSE).




[`actix-web`]: https://docs.rs/actix-web
[`juniper`]: https://docs.rs/juniper
[`juniper_actix`]: https://docs.rs/juniper_actix
[`juniper_warp`]: https://docs.rs/juniper_warp
[GraphQL]: http://graphql.org
[Juniper Book]: https://graphql-rust.github.io
[Rust]: https://www.rust-lang.org

[1]: https://github.com/graphql-rust/juniper/blob/master/juniper_actix/examples/actix_server.rs

