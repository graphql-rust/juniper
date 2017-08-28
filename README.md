# Juniper

> GraphQL server library for Rust

[![Build Status](https://travis-ci.org/graphql-rust/juniper.svg?branch=master)](https://travis-ci.org/graphql-rust/juniper)
[![Build status](https://ci.appveyor.com/api/projects/status/vsrwmsh9wobxugbs?svg=true)](https://ci.appveyor.com/project/theduke/juniper/branch/master)
[![Crates.io](https://img.shields.io/crates/v/juniper.svg?maxAge=2592000)](https://crates.io/crates/juniper)
[![Gitter chat](https://badges.gitter.im/juniper-graphql/gitter.png)](https://gitter.im/juniper-graphql)


---

[GraphQL][graphql] is a data query language developed by Facebook intended to
serve mobile and web application frontends. Juniper makes it possible to write
GraphQL servers in Rust that are type-safe and blazingly fast.

Juniper does not include a web server - instead it provides building blocks to
make integration with existing servers straightforward. It optionally provides a
pre-built integration for the [Iron][iron] and [Rocket] frameworks.

* [Cargo crate](https://crates.io/crates/juniper)
* [API Documentation](https://docs.rs/juniper)

## Installation

Add Juniper to your Cargo.toml:

```toml
[dependencies]
juniper = "0.8.1"
```

If you want Iron integration, you need to depend on the `juniper_iron` crate.
feature flag:

```toml
[dependencies]
juniper = { version = "0.8.1" }
juniper_iron = { git = "https://github.com/graphql-rust/juniper_iron" }

```

If you want Rocket integration, you need to depend on the `juniper_rocket` crate.

**Note**: Until 0.9 is released, you will need to use a Git dependency to the current master branch.

```toml
[dependencies]
juniper = { git = "https://github.com/graphql-rust/juniper" }
juniper_rocket = { git = "https://github.com/graphql-rust/juniper_rocket" }
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

graphql_object!(Human: () |&self| {
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
including polymorphism with traits and interfaces. For an example of framework
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

> Version 0.8.1 probably be re-released as 1.0 to indicate API stability.

The road to 1.0 _focuses_ on two aspects: making sure the API hasn't got any
obvious dead-ends with respect to probable future features, and improving test
coverage for general execution. There are some chores that need to be completed
as well.

* [X] Extensive execution testing
    * [X] Sending input objects and partial input objects in variables
    * [X] Sending enums in variables
    * [X] General input value type checking and validation
* [X] Improve helper macros
    * [X] `graphql_union!` helper completely missing
    * [X] `graphql_input_object!` helper completely missing
    * [X] Add support for deprecating things
    * [X] Custom enum values and descriptions
    * [X] Improved syntax for fields that can't fail resolution - make
      `FieldResult<T>` optional maybe?
* [X] Investigate asynchronous execution - implementing it is not necessary, but
  at least look at what API changes will be needed for us to hook into
  [Tokio][tokio], for example.
* [X] Larger examples to illustrate things like database access

[graphql]: http://graphql.org
[iron]: http://ironframework.io
[swschema]: http://graphql.org/docs/typesystem/
[graphql_spec]: http://facebook.github.io/graphql
[test_schema_rs]: src/tests/schema.rs
[tokio]: https://github.com/tokio-rs/tokio
[examples]: juniper_rocket/examples/
[Rocket]: https://rocket.rs
