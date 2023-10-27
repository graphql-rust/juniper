Juniper (GraphQL server library for Rust)
=========================================

[![Crates.io](https://img.shields.io/crates/v/juniper.svg?maxAge=2592000)](https://crates.io/crates/juniper)
[![Documentation](https://docs.rs/juniper/badge.svg)](https://docs.rs/juniper)
[![CI](https://github.com/graphql-rust/juniper/workflows/CI/badge.svg?branch=master "CI")](https://github.com/graphql-rust/juniper/actions?query=workflow%3ACI+branch%3Amaster)
[![Rust 1.65+](https://img.shields.io/badge/rustc-1.65+-lightgray.svg "Rust 1.65+")](https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html)

- [Juniper Book] ([current][Juniper Book] | [edge][Juniper Book edge])
- [Changelog](https://github.com/graphql-rust/juniper/blob/master/juniper/CHANGELOG.md)


[GraphQL] is a data query language developed by [Facebook] and intended to serve mobile and web application frontends.

*[Juniper]* makes it possible to write [GraphQL] servers in [Rust] that are type-safe and blazingly fast. We also try to make declaring and resolving [GraphQL] schemas as convenient as possible as [Rust] will allow.

[Juniper] doesn't include a web server - instead it provides building blocks to make integration with existing servers straightforward, including embedded [GraphiQL] and/or [GraphQL Playground] for easy debugging.




## Getting Started

The best place to get started is [Juniper Book], which contains guides with plenty of examples, covering all features of [Juniper].

To get started quickly and get a feel for Juniper, check out the ["Quickstart" section][1].

For specific information about macros, types and the [Juniper] API, the [API docs][Juniper] is the best place to look.




## Features

[Juniper] supports the full [GraphQL] query language according to [October 2021 GraphQL specification](https://spec.graphql.org/October2021), including interfaces, unions, schema introspection, and validations. It does not, however, support the schema language.

As an exception to other [GraphQL] libraries for other languages, [Juniper] builds non-`null` types by default. A field of type `Vec<Episode>` will be converted into `[Episode!]!`. The corresponding Rust type for e.g. `[Episode]` would be `Option<Vec<Option<Episode>>>`.




## Integrations


### Data types

[Juniper] has automatic integration with some very common [Rust] crates to make building schemas a breeze. The types from these crates will be usable in your schemas automatically:
- [`bigdecimal`] (feature gated)
- [`bson`]
- [`chrono`] (feature gated)
- [`chrono-tz`] (feature gated)
- [`rust_decimal`] (feature gated)
- [`time`] (feature gated)
- [`url`]
- [`uuid`]


### Web servers

- [`actix-web`] ([`juniper_actix`] crate)
- [`hyper`] ([`juniper_hyper`] crate)
- [`iron`] ([`juniper_iron`] crate)
- [`rocket`] ([`juniper_rocket`] crate)
- [`warp`] ([`juniper_warp`] crate)




## API Stability

[Juniper] has not reached 1.0 yet, thus some API instability should be expected.




## License

This project is licensed under [BSD 2-Clause License](https://github.com/graphql-rust/juniper/blob/master/juniper/LICENSE).




[`actix-web`]: https://docs.rs/actix-web
[`bigdecimal`]: https://docs.rs/bigdecimal
[`bson`]: https://docs.rs/bson
[`chrono`]: https://docs.rs/chrono
[`chrono-tz`]: https://docs.rs/chrono-tz
[`juniper_actix`]: https://docs.rs/juniper_actix
[`juniper_hyper`]: https://docs.rs/juniper_hyper
[`juniper_iron`]: https://docs.rs/juniper_iron
[`juniper_rocket`]: https://docs.rs/juniper_rocket
[`juniper_warp`]: https://docs.rs/juniper_warp
[`hyper`]: https://docs.rs/hyper
[`iron`]: https://docs.rs/iron
[`rocket`]: https://docs.rs/rocket
[`rust_decimal`]: https://docs.rs/rust_decimal
[`time`]: https://docs.rs/time
[`url`]: https://docs.rs/url
[`uuid`]: https://docs.rs/uuid
[`warp`]: https://docs.rs/warp
[Facebook]: https://facebook.com
[GraphiQL]: https://github.com/graphql/graphiql
[GraphQL]: http://graphql.org
[GraphQL Playground]: https://github.com/graphql/graphql-playground
[Juniper]: https://docs.rs/juniper
[Juniper Book]: https://graphql-rust.github.io
[Juniper Book edge]: https://graphql-rust.github.io/juniper/master
[Rust]: https://www.rust-lang.org

[1]: https://graphql-rust.github.io/quickstart.html
