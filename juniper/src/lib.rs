/*!

# GraphQL

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
guides with plenty of examples, covering all features of Juniper.

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
[book]: https://graphql-rust.github.io/
[book_quickstart]: https://graphql-rust.github.io/quickstart.html
[docsrs]: https://docs.rs/juniper

[uuid]: https://crates.io/crates/uuid
[url]: https://crates.io/crates/url
[chrono]: https://crates.io/crates/chrono

*/
#![warn(missing_docs)]

extern crate serde;
#[macro_use]
extern crate serde_derive;

#[cfg(any(test, feature = "expose-test-schema"))]
extern crate serde_json;

extern crate fnv;
extern crate indexmap;

#[cfg(any(test, feature = "chrono"))]
extern crate chrono;

#[cfg(any(test, feature = "url"))]
extern crate url;

#[cfg(any(test, feature = "uuid"))]
extern crate uuid;

// Depend on juniper_codegen and re-export everything in it.
// This allows users to just depend on juniper and get the derive functionality automatically.
#[allow(unused_imports)]
#[macro_use]
extern crate juniper_codegen;
#[doc(hidden)]
pub use juniper_codegen::*;

#[macro_use]
mod value;
#[macro_use]
mod macros;
mod ast;
mod executor;
pub mod parser;
mod schema;
mod types;
mod util;
mod validation;
// This needs to be public until docs have support for private modules:
// https://github.com/rust-lang/cargo/issues/1520
pub mod http;
pub mod integrations;
// TODO: remove this alias export in 0.10. (breaking change)
pub use http::graphiql;

#[cfg(all(test, not(feature = "expose-test-schema")))]
mod tests;
#[cfg(feature = "expose-test-schema")]
pub mod tests;

#[cfg(test)]
mod executor_tests;

// Needs to be public because macros use it.
pub use util::to_camel_case;

use executor::execute_validated_query;
use parser::{parse_document_source, ParseError, Spanning};
use validation::{validate_input_values, visit_all_rules, ValidatorContext};

pub use ast::{FromInputValue, InputValue, Selection, ToInputValue, Type};
pub use executor::{Context, ExecutionError, ExecutionResult, Executor, FieldError, FieldResult,
                   FromContext, IntoResolvable, Registry, Variables};
pub use executor::{Applies, LookAheadArgument, LookAheadSelection, LookAheadValue, LookAheadMethods};
pub use schema::model::RootNode;
pub use types::base::{Arguments, GraphQLType, TypeKind};
pub use types::scalars::{EmptyMutation, ID};
pub use validation::RuleError;
pub use value::Value;

pub use schema::meta;

/// An error that prevented query execution
#[derive(Debug, PartialEq)]
#[allow(missing_docs)]
pub enum GraphQLError<'a> {
    ParseError(Spanning<ParseError<'a>>),
    ValidationError(Vec<RuleError>),
    NoOperationProvided,
    MultipleOperationsProvided,
    UnknownOperationName,
}

/// Execute a query in a provided schema
pub fn execute<'a, CtxT, QueryT, MutationT>(
    document_source: &'a str,
    operation_name: Option<&str>,
    root_node: &RootNode<QueryT, MutationT>,
    variables: &Variables,
    context: &CtxT,
) -> Result<(Value, Vec<ExecutionError>), GraphQLError<'a>>
where
    QueryT: GraphQLType<Context = CtxT>,
    MutationT: GraphQLType<Context = CtxT>,
{
    let document = parse_document_source(document_source)?;

    {
        let errors = validate_input_values(variables, &document, &root_node.schema);

        if !errors.is_empty() {
            return Err(GraphQLError::ValidationError(errors));
        }
    }

    {
        let mut ctx = ValidatorContext::new(&root_node.schema, &document);
        visit_all_rules(&mut ctx, &document);

        let errors = ctx.into_errors();
        if !errors.is_empty() {
            return Err(GraphQLError::ValidationError(errors));
        }
    }

    execute_validated_query(document, operation_name, root_node, variables, context)
}

impl<'a> From<Spanning<ParseError<'a>>> for GraphQLError<'a> {
    fn from(f: Spanning<ParseError<'a>>) -> GraphQLError<'a> {
        GraphQLError::ParseError(f)
    }
}
