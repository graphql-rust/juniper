# Juniper

> GraphQL server library for Rust

[![build status](https://img.shields.io/travis/mhallin/juniper.svg?maxAge=2592000&style=flat-square)](https://travis-ci.org/mhallin/juniper)
[![Crates.io](https://img.shields.io/crates/v/juniper.svg?maxAge=2592000&style=flat-square)](https://crates.io/crates/juniper)
[![Coveralls](https://img.shields.io/coveralls/jekyll/jekyll.svg?maxAge=2592000&style=flat-square)](https://coveralls.io/github/mhallin/juniper)

---

[GraphQL][graphql] is a data query language developed by Facebook intended to
serve mobile and web application frontends. Juniper makes it possible to write
GraphQL servers in Rust that are type-safe and blazingly fast.

Juniper does not include a web server - instead it provides building blocks to
make integration with existing servers straightforward. It optionally provides a
pre-built integration for the [Iron framework][iron].

## Installation

Add Juniper to your Cargo.toml:

```toml
[dependencies]
juniper = "0.5.1"
```

If you want the Iron integration enabled, you need to enable the `iron-handlers`
feature flag:

```toml
[dependencies]
juniper = { version = "0.5.1", features = ["iron-handlers"] }
```

## Building schemas

GraphQL turns the REST paradigm as it's usually implemented on its head: instead
of providing a fixed structure of all types and relations in the system, GraphQL
defines a _schema_ which your users can query. The schema defines all types,
fields, and relations available, while the query defines which fields and
relations a user is interested in.

Juniper expects you to already have the types you want to expose in GraphQL as
Rust data types. Other than that, it doesn't make any assumptions whether they
are stored in a database or just in memory. Exposing a type is a matter of
implementing the `GraphQLType` for your type. To make things a bit easier,
Juniper comes with a set of macros that help you do this, based on what kind of
type you want to expose. Let's look at how one could expose parts of the [Star
Wars Schema][swschema]:

```rust
#[macro_use] extern crate juniper;

use juniper::FieldResult;

enum Episode {
    NewHope,
    Empire,
    Jedi,
}

struct Human {
    id: String,
    name: String,
    appears_in: Vec<Episode>,
    home_planet: String,
}

graphql_enum!(Episode {
    Episode::NewHope => "NEW_HOPE",
    Episode::Empire => "EMPIRE",
    Episode::Jedi => "JEDI",
});

graphql_object!(Human: () as "Human" |&self| {
    description: "A humanoid creature in the Star Wars universe"

    // Field resolver methods look almost like ordinary methods. The macro picks
    // up arguments and return types for the introspection schema, and verifies
    // it during compilation.
    field id() -> FieldResult<&String> {
        Ok(&self.id)
    }

    field name() -> FieldResult<&String> {
        Ok(&self.name)
    }

    field appears_in() -> FieldResult<&Vec<Episode>> {
        Ok(&self.appears_in)
    }

    field home_planet() -> FieldResult<&String> {
        Ok(&self.home_planet)
    }
});
```

You can find the full example in [src/tests/schema.rs][test_schema_rs],
including polymorphism with traits and interfaces. For an example of the Iron
integration, see the [examples folder][examples].

## Features

Juniper supports the full GraphQL query language according to the
[specification][graphql_spec], including the introspective schema and all
validations. It does not, however, support the schema language.

As an exception to other GraphQL libraries for other languages, Juniper builds
non-null types by default. A field of type `Vec<Episode>` will be converted into
`[Episode!]!`. The corresponding Rust type for e.g. `[Episode]` would be
`Option<Vec<Option<Episode>>>`.

## API Stability

Juniper has not reached 1.0 yet, thus some API instability should be expected.

## 1.0 Roadmap

The road to 1.0 _focuses_ on two aspects: making sure the API hasn't got any
obvious dead-ends with respect to probable future features, and improving test
coverage for general execution. There are some chores that need to be completed
as well.

* [ ] Extensive execution testing
    * [ ] Sending input objects and partial input objects in variables
    * [ ] Sending enums in variables
    * [ ] General input value type checking and validation
* [ ] Improve helper macros
    * [ ] `graphql_union!` helper completely missing
    * [ ] Add support for deprecating things
    * [ ] Custom enum values and descriptions
    * [ ] Improved syntax for fields that can't fail resolution - make
      `FieldResult<T>` optional maybe?
* [ ] Investigate asynchronous execution - implementing it is not necessary, but
  at least look at what API changes will be needed for us to hook into
  [Tokio][tokio], for example.
* [ ] Larger examples to illustrate things like database access

[graphql]: http://graphql.org
[iron]: http://ironframework.io
[swschema]: http://graphql.org/docs/typesystem/
[graphql_spec]: http://facebook.github.io/graphql
[test_schema_rs]: src/tests/schema.rs
[tokio]: https://github.com/tokio-rs/tokio
[examples]: examples/
