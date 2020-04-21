<img src="https://github.com/graphql-rust/juniper/raw/master/assets/logo/juniper-dark-word.png" alt="Juniper" width="500" />

> GraphQL server library for Rust

[![Build Status](https://dev.azure.com/graphql-rust/GraphQL%20Rust/_apis/build/status/graphql-rust.juniper)](https://dev.azure.com/graphql-rust/GraphQL%20Rust/_build/latest?definitionId=1)
[![codecov](https://codecov.io/gh/graphql-rust/juniper/branch/master/graph/badge.svg)](https://codecov.io/gh/graphql-rust/juniper)
[![Crates.io](https://img.shields.io/crates/v/juniper.svg?maxAge=2592000)](https://crates.io/crates/juniper)
[![Gitter chat](https://badges.gitter.im/juniper-graphql/gitter.svg)](https://gitter.im/juniper-graphql)

---

[GraphQL][graphql] is a data query language developed by Facebook intended to
serve mobile and web application frontends.

_Juniper_ makes it possible to write GraphQL servers in Rust that are
type-safe and blazingly fast. We also try to make declaring and resolving
GraphQL schemas as convenient as Rust will allow.

Juniper does not include a web server - instead it provides building blocks to
make integration with existing servers straightforward. It optionally provides a
pre-built integration for the [Actix][actix], [Hyper][hyper], [Iron][iron], [Rocket], and [Warp][warp] frameworks, including
embedded [Graphiql][graphiql] and [GraphQL Playground][playground] for easy debugging.

- [Cargo crate](https://crates.io/crates/juniper)
- [API Reference][docsrs]
- [Book][book]: Guides and Examples ([current][book] | [master][book_master])

The book is also available for the master branch and older versions published after 0.11.1. See the [book index][book_index].


## Getting Started

The best place to get started is the [Juniper Book][book], which contains
guides with plenty of examples, covering all features of Juniper. (very much WIP)

To get started quickly and get a feel for Juniper, check out the
[Quickstart][book_quickstart] section.

For specific information about macros, types and the Juniper api, the
[API Reference][docsrs] is the best place to look.

You can also check out [src/tests/schema.rs][test_schema_rs] to see a complex
schema including polymorphism with traits and interfaces.
For an example of web framework integration,
see the [actix][actix_examples], [hyper][hyper_examples], [rocket][rocket_examples], [iron][iron_examples], and [warp][warp_examples] examples folders.

## Features

Juniper supports the full GraphQL query language according to the
[specification][graphql_spec], including interfaces, unions, schema
introspection, and validations.
It does not, however, support the schema language. Consider using [juniper-from-schema][] for generating code from a schema file.

As an exception to other GraphQL libraries for other languages, Juniper builds
non-null types by default. A field of type `Vec<Episode>` will be converted into
`[Episode!]!`. The corresponding Rust type for e.g. `[Episode]` would be
`Option<Vec<Option<Episode>>>`.

## Integrations

### Data types

Juniper has automatic integration with some very common Rust crates to make
building schemas a breeze. The types from these crates will be usable in
your Schemas automatically.

- [uuid][uuid]
- [url][url]
- [chrono][chrono]
- [bson][bson]

### Web Frameworks

- [actix][actix]
- [hyper][hyper]
- [rocket][rocket]
- [iron][iron]
- [warp][warp]

## Guides & Examples

- [Juniper + actix-web example](https://github.com/actix/examples/tree/master/juniper)

## API Stability

Juniper has not reached 1.0 yet, thus some API instability should be expected.

[actix]: https://actix.rs/
[graphql]: http://graphql.org
[graphiql]: https://github.com/graphql/graphiql
[playground]: https://github.com/prisma/graphql-playground
[iron]: http://ironframework.io
[graphql_spec]: http://facebook.github.io/graphql
[test_schema_rs]: https://github.com/graphql-rust/juniper/blob/master/juniper/src/tests/schema.rs
[tokio]: https://github.com/tokio-rs/tokio
[actix_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_actix/examples
[hyper_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_hyper/examples
[rocket_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_rocket/examples
[iron_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_iron/examples
[hyper]: https://hyper.rs
[rocket]: https://rocket.rs
[book]: https://graphql-rust.github.io/juniper/current
[book_master]: https://graphql-rust.github.io/juniper/master
[book_index]: https://graphql-rust.github.io/juniper
[book_quickstart]: https://graphql-rust.github.io/juniper/current/quickstart.html
[docsrs]: https://docs.rs/juniper
[warp]: https://github.com/seanmonstar/warp
[warp_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_warp/examples
[uuid]: https://crates.io/crates/uuid
[url]: https://crates.io/crates/url
[chrono]: https://crates.io/crates/chrono
[bson]: https://crates.io/crates/bson
[juniper-from-schema]: https://github.com/davidpdrsn/juniper-from-schema
