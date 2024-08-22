Introduction
============

> [GraphQL] is a query language for APIs and a runtime for fulfilling those queries with your existing data. [GraphQL] provides a complete and understandable description of the data in your API, gives clients the power to ask for exactly what they need and nothing more, makes it easier to evolve APIs over time, and enables powerful developer tools.

[Juniper] is a library for creating [GraphQL] servers in [Rust]. Build type-safe and fast API servers with minimal boilerplate and configuration (we do try to make declaring and resolving [GraphQL] schemas as convenient as [Rust] will allow).

[Juniper] doesn't include a web server itself, instead, it provides building blocks to make integration with existing web servers straightforward. It optionally provides a pre-built integration for some widely used web server frameworks in [Rust] ecosystem.

- [Cargo crate](https://crates.io/crates/juniper)
- [API reference][`juniper`]




## Features

[Juniper] supports the full GraphQL query language according to the [specification (October 2021)][GraphQL spec].

> **NOTE**: As an exception to other [GraphQL] libraries for other languages, [Juniper] builds non-`null` types by default. A field of type `Vec<Episode>` will be converted into `[Episode!]!`. The corresponding Rust type for a `null`able `[Episode]` would be `Option<Vec<Option<Episode>>>` instead.




## Integrations


### Types

[Juniper] provides out-of-the-box integration for some very common [Rust] crates to make building schemas a breeze. The types from these crates will be usable in your schemas automatically after enabling the correspondent self-titled [Cargo feature]:
- [`bigdecimal`]
- [`bson`]
- [`chrono`], [`chrono-tz`]
- [`jiff`]
- [`rust_decimal`]
- [`time`]
- [`url`]
- [`uuid`]




### Web server frameworks

- [`actix-web`] ([`juniper_actix`] crate)
- [`axum`] ([`juniper_axum`] crate)
- [`hyper`] ([`juniper_hyper`] crate)
- [`rocket`] ([`juniper_rocket`] crate)
- [`warp`] ([`juniper_warp`] crate)




## API stability

[Juniper] has not reached 1.0 yet, thus some API instability should be expected.




[`actix-web`]: https://docs.rs/actix-web
[`axum`]: https://docs.rs/axum
[`bigdecimal`]: https://docs.rs/bigdecimal
[`bson`]: https://docs.rs/bson
[`chrono`]: https://docs.rs/chrono
[`chrono-tz`]: https://docs.rs/chrono-tz
[`jiff`]: https://docs.rs/jiff
[`juniper`]: https://docs.rs/juniper
[`juniper_actix`]: https://docs.rs/juniper_actix
[`juniper_axum`]: https://docs.rs/juniper_axum
[`juniper_hyper`]: https://docs.rs/juniper_hyper
[`juniper_rocket`]: https://docs.rs/juniper_rocket
[`juniper_warp`]: https://docs.rs/juniper_warp
[`hyper`]: https://docs.rs/hyper
[`rocket`]: https://docs.rs/rocket
[`rust_decimal`]: https://docs.rs/rust_decimal
[`time`]: https://docs.rs/time
[`url`]: https://docs.rs/url
[`uuid`]: https://docs.rs/uuid
[`warp`]: https://docs.rs/warp
[Cargo feature]: https://doc.rust-lang.org/cargo/reference/features.html
[GraphQL]: https://graphql.org
[GraphQL spec]: https://spec.graphql.org/October2021
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org
