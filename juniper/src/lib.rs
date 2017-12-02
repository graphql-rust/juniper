/*!

# GraphQL

[GraphQL][1] is a data query language developed by Facebook intended to serve
mobile and web application frontends. A server provides a schema, containing
types and fields that applications can query. Queries are hierarchical,
composable, and statically typed. Schemas are introspective, which lets clients
statically verify their queries against a server without actually executing
them.

This library provides data types and traits to expose Rust types in a GraphQL
schema, as well as an optional integration into the [Iron framework][Iron] and
[Rocket]. It tries to keep the number of dynamic operations to a minimum, and
give you as the schema developer the control of the query execution path.

## Exposing data types

The `GraphQLType` trait is the primary interface towards application developers.
By implementing this trait, you can expose your types as either objects, enums,
interfaces, unions, or scalars.

However, due to the dynamic nature of GraphQL's type system, doing this
manually is a bit tedious, especially in order to do it in a fully type safe
manner.

The library provides two methods of mapping your Rust data types to GraphQL schemas: custom derive
implementations and macros.

```rust
# use std::collections::HashMap;
# #[macro_use] extern crate juniper;
use juniper::{Context, FieldResult};

struct User { id: String, name: String, friend_ids: Vec<String>  }
struct QueryRoot;
struct Database { users: HashMap<String, User> }

impl Context for Database {}

// GraphQL objects can access a "context object" during execution. Use this
// object to provide e.g. database access to the field accessors. This object
// must implement the `Context` trait. If you don't need a context, use the
// empty tuple `()` to indicate this.
//
// In this example, we use the Database struct as our context.
graphql_object!(User: Database |&self| {

    // Expose a simple field as a GraphQL string.
    field id() -> &String {
        &self.id
    }

    field name() -> &String {
        &self.name
    }

    // FieldResult<T> is an alias for Result<T, FieldError>, which can be
    // converted to from anything that implements std::fmt::Display - simply
    // return an error with a string using the ? operator from this method and
    // it will be correctly inserted into the execution response.
    field secret() -> FieldResult<&String> {
        Err("Can't touch this".to_owned())?
    }

    // Field accessors can optionally take an "executor" as their first
    // argument. This object can help guide query execution and provides
    // access to the context instance.
    //
    // In this example, the context is used to convert the friend_ids array
    // into actual User objects.
    field friends(&executor) -> Vec<&User> {
        self.friend_ids.iter()
            .filter_map(|id| executor.context().users.get(id))
            .collect()
    }
});

// The context object is passed down to all referenced types - all your exposed
// types need to have the same context type.
graphql_object!(QueryRoot: Database |&self| {

    // Arguments work just like they do on functions.
    field user(&executor, id: String) -> Option<&User> {
        executor.context().users.get(&id)
    }
});

# fn main() { }
```

Adding per type, field, and argument documentation is possible directly from
this macro. For more in-depth information on how to expose fields and types, see
the [`graphql_object!`][3] macro.

### Built-in object type integrations

Juniper has [built-in integrations][object_integrations] for converting existing object types to
GraphQL objects for popular crates.

## Integrating with web servers

The most obvious usecase is to expose the GraphQL schema over an HTTP endpoint.
To support this, Juniper offers additional crates that integrate with popular web frameworks.

* [juniper_iron][juniper_iron]: Handlers for [Iron][Iron]
* [juniper_rocket][juniper_rocket]: Handlers for [Rocket][Rocket]

[1]: http://graphql.org
[3]: macro.graphql_object!.html
[Iron]: http://ironframework.io
[Rocket]: https://rocket.rs
[object_integrations]: integrations/index.html

*/
#![warn(missing_docs)]

extern crate serde;
#[macro_use]
extern crate serde_derive;

#[cfg(any(test, feature = "expose-test-schema"))]
extern crate serde_json;

extern crate fnv;
extern crate ordermap;

#[cfg(any(test, feature = "chrono"))]
extern crate chrono;

#[cfg(any(test, feature = "url"))]
extern crate url;

#[cfg(any(test, feature = "uuid"))]
extern crate uuid;

// If the "codegen" feature is enabled, depend on juniper_codegen and re-export everything in it.
// This allows users to just depend on juniper and get the derive funcationality automatically.
#[cfg(feature = "codegen")]
#[allow(unused_imports)]
#[macro_use]
extern crate juniper_codegen;

#[cfg(feature = "codegen")]
#[doc(hidden)]
pub use juniper_codegen::*;


#[macro_use]
mod value;
#[macro_use]
mod macros;
mod ast;
pub mod parser;
mod types;
mod schema;
mod validation;
mod util;
mod executor;
// This needs to be public until docs have support for private modules:
// https://github.com/rust-lang/cargo/issues/1520
pub mod integrations;
pub mod graphiql;
pub mod http;
#[macro_use]
mod result_ext;

#[cfg(all(test, not(feature = "expose-test-schema")))]
mod tests;
#[cfg(feature = "expose-test-schema")]
pub mod tests;

#[cfg(test)]
mod executor_tests;

// Needs to be public because macros use it.
pub use util::to_camel_case;

use parser::{parse_document_source, ParseError, Spanning};
use validation::{validate_input_values, visit_all_rules, ValidatorContext};
use executor::execute_validated_query;

pub use ast::{FromInputValue, InputValue, Selection, ToInputValue, Type};
pub use value::Value;
pub use types::base::{Arguments, GraphQLType, TypeKind};
pub use executor::{Context, ExecutionError, ExecutionResult, Executor, FieldError, FieldResult,
                   FromContext, IntoResolvable, Registry, Variables};
pub use validation::RuleError;
pub use types::scalars::{EmptyMutation, ID};
pub use schema::model::RootNode;
pub use result_ext::ResultExt;

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
    let document = try!(parse_document_source(document_source));

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


