<img src="https://github.com/graphql-rust/juniper/blob/master/assets/logo/juniper-dark-word.png" alt="Juniper" width="500" />


> GraphQL server library for Rust

[![Build Status](https://travis-ci.org/graphql-rust/juniper.svg?branch=master)](https://travis-ci.org/graphql-rust/juniper)
[![Build status](https://ci.appveyor.com/api/projects/status/xav6tor6biu617uu?svg=true)](https://ci.appveyor.com/project/theduke/juniper)
[![codecov](https://codecov.io/gh/graphql-rust/juniper/branch/master/graph/badge.svg)](https://codecov.io/gh/graphql-rust/juniper)
[![Crates.io](https://img.shields.io/crates/v/juniper.svg?maxAge=2592000)](https://crates.io/crates/juniper)
[![Gitter chat](https://badges.gitter.im/juniper-graphql/gitter.png)](https://gitter.im/juniper-graphql)


---

[GraphQL][graphql] is a data query language developed by Facebook intended to
serve mobile and web application frontends. 

*Juniper* makes it possible to write GraphQL servers in Rust that are 
type-safe and blazingly fast. We also try to make declaring and resolving 
GraphQL schemas as convenient as possible as Rust will allow.

Juniper does not include a web server - instead it provides building blocks to
make integration with existing servers straightforward. It optionally provides a
pre-built integration for the [Iron][iron] and [Rocket] frameworks, including
embedded [Graphiql][graphiql] for easy debugging.

* [Cargo crate](https://crates.io/crates/juniper)
* [API Reference][docsrs]
* [Book][book]: Guides and Examples


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
see the [rocket][rocket_examples] and [iron][iron_examples] examples folders.


## Features

Juniper supports the full GraphQL query language according to the
[specification][graphql_spec], including interfaces, unions, schema 
introspection, and validations.  
It does not, however, support the schema language.

As an exception to other GraphQL libraries for other languages, Juniper builds
non-null types by default. A field of type `Vec<Episode>` will be converted into
`[Episode!]!`. The corresponding Rust type for e.g. `[Episode]` would be
`Option<Vec<Option<Episode>>>`.

## Integrations

### Data types

Juniper has automatic integration with some very common Rust crates to make
building schemas a breeze. The types from these crates will be usable in
your Schemas automatically.

* [uuid][uuid]
* [url][url]
* [chrono][chrono]

### Web Frameworks

* [rocket][rocket]
* [iron][iron]

## Guides & Examples

* [Juniper + actix-web example](https://github.com/actix/examples/tree/master/juniper)


## API Stability

Juniper has not reached 1.0 yet, thus some API instability should be expected.

[graphql]: http://graphql.org
[graphiql]: https://github.com/graphql/graphiql
[iron]: http://ironframework.io
[graphql_spec]: http://facebook.github.io/graphql
[test_schema_rs]: https://github.com/graphql-rust/juniper/blob/master/juniper/src/tests/schema.rs
[tokio]: https://github.com/tokio-rs/tokio
[rocket_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_rocket/examples
[iron_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_iron/examples
[Rocket]: https://rocket.rs
[book]: https://graphql-rust.github.io
[book_quickstart]: https://graphql-rust.github.io/quickstart.html
[docsrs]: https://docs.rs/juniper

[uuid]: https://crates.io/crates/uuid
[url]: https://crates.io/crates/url
[chrono]: https://crates.io/crates/chrono

