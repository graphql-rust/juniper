/*!

# GraphQL

[GraphQL][1] is a data query language developed by Facebook intended to serve
mobile and web application frontends. A server provides a schema, containing
types and fields that applications can query. Queries are hierarchical,
composable, and statically typed. Schemas are introspective, which lets clients
statically verify their queries against a server without actually executing
them.

This library provides data types and traits to expose Rust types in a GraphQL
schema, as well as an optional integration into the [Iron framework][2] and
[Rocket]. It tries to keep the number of dynamic operations to a minimum, and
give you as the schema developer the control of the query execution path.

Juniper only depends on `serde` and `serde_derive` by default, making it
lightweight and easy to drop into any project. If you enable any of the
optional framework integrations, it will naturally depend on those frameworks
too.

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
# use std::collections::HashMap;
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

    // FieldResult<T> is an alias for Result<T, String> - simply return
    // a string from this method and it will be correctly inserted into
    // the execution response.
    field secret() -> FieldResult<&String> {
        Err("Can't touch this".to_owned())
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

## Integrating with Iron

The most obvious usecase is to expose the GraphQL schema over an HTTP endpoint.
To support this, the library provides optional and customizable handlers for
both Iron and Rocket.

For example, continuing from the schema created above and using Iron to expose
the schema on an HTTP endpoint supporting both GET and POST requests:

```rust,no_run
extern crate iron;
# #[macro_use] extern crate juniper;
# use std::collections::HashMap;

use iron::prelude::*;
use juniper::iron_handlers::GraphQLHandler;
use juniper::{Context, EmptyMutation};

# use juniper::FieldResult;
#
# struct User { id: String, name: String, friend_ids: Vec<String>  }
# struct QueryRoot;
# struct Database { users: HashMap<String, User> }
#
# graphql_object!(User: Database |&self| {
#     field id() -> FieldResult<&String> {
#         Ok(&self.id)
#     }
#
#     field name() -> FieldResult<&String> {
#         Ok(&self.name)
#     }
#
#     field friends(&executor) -> FieldResult<Vec<&User>> {
#         Ok(self.friend_ids.iter()
#             .filter_map(|id| executor.context().users.get(id))
#             .collect())
#     }
# });
#
# graphql_object!(QueryRoot: Database |&self| {
#     field user(&executor, id: String) -> FieldResult<Option<&User>> {
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

impl Context for Database {}

fn main() {
    // GraphQLHandler takes a context factory function, the root object,
    // and the mutation object. If we don't have any mutations to expose, we
    // can use the empty tuple () to indicate absence.
    let graphql_endpoint = GraphQLHandler::new(
        context_factory, QueryRoot, EmptyMutation::<Database>::new());

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
[Rocket]: https://rocket.rs

*/

#![cfg_attr(feature="nightly", feature(test))]
#![warn(missing_docs)]

#![cfg_attr(feature="rocket-handlers", feature(plugin))]
#![cfg_attr(feature="rocket-handlers", plugin(rocket_codegen))]
#[cfg(feature="rocket-handlers")] extern crate rocket;

#[cfg(feature="nightly")] extern crate test;
#[cfg(feature="iron-handlers")] #[macro_use(itry)] extern crate iron;
#[cfg(feature="iron-handlers")] extern crate urlencoded;
#[cfg(test)] extern crate iron_test;
extern crate serde;
#[macro_use] extern crate serde_derive;

#[cfg(any(feature="iron-handlers", feature="rocket-handlers"))] extern crate serde_json;

use std::borrow::Cow;

#[macro_use] mod macros;
mod ast;
pub mod parser;
mod value;
mod types;
mod schema;
mod validation;
mod executor;
mod integrations;
pub mod graphiql;
pub mod http;
#[macro_use] mod result_ext;

#[cfg(all(test, not(feature="expose-test-schema")))] mod tests;
#[cfg(feature="expose-test-schema")] pub mod tests;

#[cfg(test)] mod executor_tests;

use parser::{parse_document_source, ParseError, Spanning};
use validation::{ValidatorContext, visit_all_rules, validate_input_values};
use executor::execute_validated_query;

pub use ast::{ToInputValue, FromInputValue, InputValue, Type, Selection};
pub use value::Value;
pub use types::base::{Arguments, GraphQLType, TypeKind};
pub use executor::{
    Executor, ExecutionError, Registry,
    Context, FromContext, IntoResolvable,
    FieldResult, ExecutionResult, Variables,
};
pub use validation::RuleError;
pub use types::scalars::{EmptyMutation, ID};
pub use schema::model::RootNode;
pub use result_ext::ResultExt;

pub use schema::meta;

#[cfg(feature="iron-handlers")] pub use integrations::iron_handlers;
#[cfg(feature="rocket-handlers")] pub use integrations::rocket_handlers;

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
)
    -> Result<(Value, Vec<ExecutionError>), GraphQLError<'a>>
    where QueryT: GraphQLType<Context=CtxT>,
          MutationT: GraphQLType<Context=CtxT>,
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

#[doc(hidden)]
pub fn to_camel_case<'a>(s: &'a str) -> Cow<'a, str> {
    let mut dest = Cow::Borrowed(s);

    for (i, part) in s.split('_').enumerate() {
        if i > 0 && part.len() == 1 {
            dest += Cow::Owned(part.to_uppercase());
        }
        else if i > 0 && part.len() > 1 {
            let first = part.chars().next().unwrap().to_uppercase().collect::<String>();
            let second = &part[1..];

            dest += Cow::Owned(first);
            dest += second;
        }
        else if i == 0 {
            dest = Cow::Borrowed(part);
        }
    }

    dest
}

#[test]
fn test_to_camel_case() {
    assert_eq!(&to_camel_case("test")[..], "test");
    assert_eq!(&to_camel_case("_test")[..], "Test");
    assert_eq!(&to_camel_case("first_second")[..], "firstSecond");
    assert_eq!(&to_camel_case("first_")[..], "first");
    assert_eq!(&to_camel_case("a_b_c")[..], "aBC");
    assert_eq!(&to_camel_case("a_bc")[..], "aBc");
    assert_eq!(&to_camel_case("a_b")[..], "aB");
    assert_eq!(&to_camel_case("a")[..], "a");
    assert_eq!(&to_camel_case("")[..], "");
}
