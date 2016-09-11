/*!

# GraphQL

[GraphQL][1] is a data query language developed by Facebook intended to serve
mobile and web application frontends. A server provides a schema, containing
types and fields that applications can query. Queries are hierarchical,
composable, and statically typed. Schemas are introspective, which lets clients
statically verify their queries against a server without actually executing
them.

This library provides data types and traits to expose Rust types in a GraphQL
schema, as well as an optional integration into the [Iron framework][2]. It
tries to keep the number of dynamic operations to a minimum, and give you as the
schema developer the control of the query execution path.

## Exposing data types

The `GraphQLType` trait is the primary interface towards application developers.
By deriving this trait, you can expose your types as either objects, enums,
interfaces, unions, or scalars.

However, due to the dynamic nature of GraphQL's type system, deriving this trait
manually is a bit tedious, especially in order to do it in a fully type safe
manner. To help with this task, this library provides a couple of macros; the
most common one being `graphql_object!`. Use this macro to expose your already
existing object types as GraphQL objects:

```rust
#[macro_use] extern crate juniper;
use juniper::FieldResult;
# use std::collections::HashMap;

struct User { id: String, name: String, friend_ids: Vec<String>  }
struct QueryRoot;
struct Database { users: HashMap<String, User> }

// GraphQL objects can access a "context object" during execution. Use this
// object to provide e.g. database access to the field accessors.
//
// In this example, we use the Database struct as our context.
graphql_object!(User: Database as "User" |&self| {

    // Expose a simple field as a GraphQL string.
    // FieldResult<T> is an alias for Result<T, String> - simply return
    // a string from this method and it will be correctly inserted into
    // the execution response.
    field id() -> FieldResult<&String> {
        Ok(&self.id)
    }

    field name() -> FieldResult<&String> {
        Ok(&self.name)
    }

    // Field accessors can optionally take an "executor" as their first
    // argument. This object can help guide query execution and provides
    // access to the context instance.
    //
    // In this example, the context is used to convert the friend_ids array
    // into actual User objects.
    field friends(&mut executor) -> FieldResult<Vec<&User>> {
        Ok(self.friend_ids.iter()
            .filter_map(|id| executor.context().users.get(id))
            .collect())
    }
});

// The context object is passed down to all referenced types - all your exposed
// types need to have the same context type.
graphql_object!(QueryRoot: Database as "Query" |&self| {

    // Arguments work just like they do on functions.
    field user(&mut executor, id: String) -> FieldResult<Option<&User>> {
        Ok(executor.context().users.get(&id))
    }
});

# fn main() { }
```

Adding per type, field, and argument documentation is possible directly from
this macro. For more in-depth information on how to expose fields and types, see
the [`graphql_object!`][3] macro.

## Integrating with Iron

The most obvious usecase is to expose the GraphQL schema over an HTTP endpoint.
To support this, the library provides an optional and customizable Iron handler.

For example, continuing from the schema created above:

```rust,no_run
extern crate iron;
# #[macro_use] extern crate juniper;
# use std::collections::HashMap;

use iron::prelude::*;
use juniper::iron_handlers::GraphQLHandler;

# use juniper::FieldResult;
#
# struct User { id: String, name: String, friend_ids: Vec<String>  }
# struct QueryRoot;
# struct Database { users: HashMap<String, User> }
#
# graphql_object!(User: Database as "User" |&self| {
#     field id() -> FieldResult<&String> {
#         Ok(&self.id)
#     }
#
#     field name() -> FieldResult<&String> {
#         Ok(&self.name)
#     }
#
#     field friends(&mut executor) -> FieldResult<Vec<&User>> {
#         Ok(self.friend_ids.iter()
#             .filter_map(|id| executor.context().users.get(id))
#             .collect())
#     }
# });
#
# graphql_object!(QueryRoot: Database as "Query" |&self| {
#     field user(&mut executor, id: String) -> FieldResult<Option<&User>> {
#         Ok(executor.context().users.get(&id))
#     }
# });

// This function is executed for every request. Here, we would realistically
// provide a database connection or similar. For this example, we'll be
// creating the database from scratch.
fn context_factory(_: &mut Request) -> Database {
    Database {
        users: vec![
            ( "1000".to_owned(), User {
                id: "1000".to_owned(), name: "Robin".to_owned(),
                friend_ids: vec!["1001".to_owned()] } ),
            ( "1001".to_owned(), User {
                id: "1001".to_owned(), name: "Max".to_owned(),
                friend_ids: vec!["1000".to_owned()] } ),
        ].into_iter().collect()
    }
}

fn main() {
    // GraphQLHandler takes a context factory function, the root object,
    // and the mutation object. If we don't have any mutations to expose, we
    // can use the empty tuple () to indicate absence.
    let graphql_endpoint = GraphQLHandler::new(context_factory, QueryRoot, ());

    // Start serving the schema at the root on port 8080.
    Iron::new(graphql_endpoint).http("localhost:8080").unwrap();
}

```

See the [`iron_handlers`][4] module and the [`GraphQLHandler`][5] documentation
for more information on what request methods are supported. There's also a
built-in [GraphiQL][6] handler included.

[1]: http://graphql.org
[2]: http://ironframework.io
[3]: macro.graphql_object!.html
[4]: iron_handlers/index.html
[5]: iron_handlers/struct.GraphQLHandler.html
[6]: https://github.com/graphql/graphiql

*/

#![cfg_attr(feature="nightly", feature(test))]
#![warn(missing_docs)]

extern crate rustc_serialize;

#[cfg(feature="nightly")] extern crate test;
#[cfg(feature="iron-handlers")] #[macro_use(itry, iexpect)] extern crate iron;
#[cfg(test)] extern crate iron_test;

#[macro_use] mod macros;
mod ast;
pub mod parser;
mod value;
mod types;
mod schema;
pub mod validation;
mod integrations;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use rustc_serialize::json::{ToJson, Json};

use parser::{parse_document_source, ParseError, Spanning, SourcePosition};
use types::execute_validated_query;
use validation::{RuleError, ValidatorContext, visit_all_rules};

pub use ast::{ToInputValue, FromInputValue, InputValue, Type, Selection};
pub use value::Value;
pub use types::base::{Arguments, GraphQLType, TypeKind};
pub use types::schema::{Executor, Registry, ExecutionResult, ExecutionError, FieldResult};
pub use types::scalars::ID;
pub use schema::model::RootNode;

pub use schema::meta;

#[cfg(feature="iron-handlers")] pub use integrations::iron_handlers;

/// An error that prevented query execution
#[derive(Debug, PartialEq)]
#[allow(missing_docs)]
pub enum GraphQLError<'a> {
    ParseError(Spanning<ParseError<'a>>),
    ValidationError(Vec<RuleError>),
}

/// Execute a query in a provided schema
pub fn execute<'a, CtxT, QueryT, MutationT>(
    document_source: &'a str,
    operation_name: Option<&str>,
    root_node: &RootNode<CtxT, QueryT, MutationT>,
    variables: &HashMap<String, InputValue>,
    context: &CtxT,
)
    -> Result<(Value, Vec<ExecutionError>), GraphQLError<'a>>
    where QueryT: GraphQLType<CtxT>,
          MutationT: GraphQLType<CtxT>,
{
    let document = try!(parse_document_source(document_source));

    {
        let mut ctx = ValidatorContext::new(&root_node.schema, &document);
        visit_all_rules(&mut ctx, &document);

        let errors = ctx.into_errors();
        if !errors.is_empty() {
            return Err(GraphQLError::ValidationError(errors));
        }
    }

    Ok(execute_validated_query(document, operation_name, root_node, variables, context))
}

impl<'a> From<Spanning<ParseError<'a>>> for GraphQLError<'a> {
    fn from(f: Spanning<ParseError<'a>>) -> GraphQLError<'a> {
        GraphQLError::ParseError(f)
    }
}

impl<'a> ToJson for GraphQLError<'a> {
    fn to_json(&self) -> Json {
        let errs = match *self {
            GraphQLError::ParseError(ref err) => parse_error_to_json(err),
            GraphQLError::ValidationError(ref errs) => errs.to_json(),
        };

        Json::Object(vec![
            ("errors".to_owned(), errs),
        ].into_iter().collect())
    }
}

fn parse_error_to_json(err: &Spanning<ParseError>) -> Json {
    Json::Array(vec![
        Json::Object(vec![
            ("message".to_owned(), format!("{}", err.item).to_json()),
            ("locations".to_owned(), vec![
                Json::Object(vec![
                    ("line".to_owned(), (err.start.line() + 1).to_json()),
                    ("column".to_owned(), (err.start.column() + 1).to_json())
                ].into_iter().collect()),
            ].to_json()),
        ].into_iter().collect()),
    ])
}

impl ToJson for RuleError {
    fn to_json(&self) -> Json {
        Json::Object(vec![
            ("message".to_owned(), self.message().to_json()),
            ("locations".to_owned(), self.locations().to_json()),
        ].into_iter().collect())
    }
}

impl ToJson for SourcePosition {
    fn to_json(&self) -> Json {
        Json::Object(vec![
            ("line".to_owned(), (self.line() + 1).to_json()),
            ("column".to_owned(), (self.column() + 1).to_json()),
        ].into_iter().collect())
    }
}

impl ToJson for ExecutionError {
    fn to_json(&self) -> Json {
        Json::Object(vec![
            ("message".to_owned(), self.message().to_json()),
            ("locations".to_owned(), vec![self.location().clone()].to_json()),
            ("path".to_owned(), self.path().to_json()),
        ].into_iter().collect())
    }
}

#[doc(hidden)]
pub fn to_snake_case(s: &str) -> String {
    let mut dest = String::new();

    for (i, part) in s.split('_').enumerate() {
        if i > 0 && part.len() == 1 {
            dest.push_str(&part.to_uppercase());
        }
        else if i > 0 && part.len() > 1 {
            let first = part.chars().next().unwrap().to_uppercase().collect::<String>();
            let second = &part[1..];

            dest.push_str(&first);
            dest.push_str(second);
        }
        else if i == 0 {
            dest.push_str(part);
        }
    }

    dest
}

#[test]
fn test_to_snake_case() {
    assert_eq!(&to_snake_case("test")[..], "test");
    assert_eq!(&to_snake_case("_test")[..], "Test");
    assert_eq!(&to_snake_case("first_second")[..], "firstSecond");
    assert_eq!(&to_snake_case("first_")[..], "first");
    assert_eq!(&to_snake_case("a_b_c")[..], "aBC");
    assert_eq!(&to_snake_case("a_bc")[..], "aBc");
    assert_eq!(&to_snake_case("a_b")[..], "aB");
    assert_eq!(&to_snake_case("a")[..], "a");
    assert_eq!(&to_snake_case("")[..], "");
}
