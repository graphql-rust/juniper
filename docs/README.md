# Juniper

Juniper is a [GraphQL] server library for Rust. Build type-safe and fast API
servers with minimal boilerplate and configuration.

[GraphQL][graphql] is a data query language developed by Facebook intended to
serve mobile and web application frontends. 

Juniper makes it possible to write GraphQL servers in Rust that are 
type-safe and blazingly fast. We also try to make declaring and resolving 
GraphQL schemas as convenient as possible as Rust will allow.

Juniper does not include a web server - instead it provides building blocks to
make integration with existing servers straightforward. It optionally provides a
pre-built integration for the [Iron][iron] and [Rocket] frameworks, including
embedded [Graphiql][graphiql] for easy debugging.

* [Github](https://github.com/graphql-rust/juniper)
* [Cargo crate](https://crates.io/crates/juniper)
* [API Reference][docsrs]


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


## API Stability

Juniper has not reached 1.0 yet, thus some API instability should be expected.

[graphql]: http://graphql.org
[graphiql]: https://github.com/graphql/graphiql
[iron]: http://ironframework.io
[graphql_spec]: http://facebook.github.io/graphql
[test_schema_rs]: https://github.com/graphql-rust/juniper/blob/master/src/tests/schema.rs
[tokio]: https://github.com/tokio-rs/tokio
[rocket_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_rocket/examples
[iron_examples]: https://github.com/graphql-rust/juniper/tree/master/juniper_iron/examples
[Rocket]: https://rocket.rs
[book]: https://graphql-rust.github.io/juniper-book
[book_quickstart]: https://graphql-rust.github.io/juniper-book/quickstart.html
[docsrs]: https://docs.rs/juniper

[uuid]: https://crates.io/crates/uuid
[url]: https://crates.io/crates/url
[chrono]: https://crates.io/crates/chrono
