Schema errors
=============

Rust's model of errors can be adapted for GraphQL. Rust's panic is
similar to a `FieldError`--the whole query is aborted and nothing can
be extracted (except for error related information).

Not all errors require this strict handling. Recoverable or partial errors can be put
into the GraphQL schema so the client can intelligently handle them.

To implement this approach, all errors must be partitioned into two error classes:

* Critical errors that cannot be fixed by the user (e.g. a database error).
* Recoverable errors that can be fixed by the user (e.g. invalid input data).

Critical errors are returned from resolvers as `FieldErrors` (from the previous section). Non-critical errors are part of the GraphQL schema and can be handled gracefully by clients. Similar to Rust, GraphQL allows similar error models with unions (see Unions).

### Example Input Validation (simple)

In this example, basic input validation is implemented with GraphQL
types. Strings are used to identify the problematic field name. Errors
for a particular field are also returned as a string. In this example
the string contains a server-side localized error message. However, it is also
possible to return a unique string identifier and have the client present a localized string to the user.

```rust
# extern crate juniper;
# use juniper::{graphql_object, GraphQLObject, GraphQLUnion};
#
#[derive(GraphQLObject)]
pub struct Item {
    name: String,
    quantity: i32,
}

#[derive(GraphQLObject)]
pub struct ValidationError {
    field: String,
    message: String,
}

#[derive(GraphQLObject)]
pub struct ValidationErrors {
    errors: Vec<ValidationError>,
}

#[derive(GraphQLUnion)]
pub enum GraphQLResult {
    Ok(Item),
    Err(ValidationErrors),
}

pub struct Mutation;

#[graphql_object]
impl Mutation {
    fn addItem(&self, name: String, quantity: i32) -> GraphQLResult {
        let mut errors = Vec::new();

        if !(10 <= name.len() && name.len() <= 100) {
            errors.push(ValidationError {
                field: "name".into(),
                message: "between 10 and 100".into(),
            });
        }

        if !(1 <= quantity && quantity <= 10) {
            errors.push(ValidationError {
                field: "quantity".into(),
                message: "between 1 and 10".into(),
            });
        }

        if errors.is_empty() {
            GraphQLResult::Ok(Item { name, quantity })
        } else {
            GraphQLResult::Err(ValidationErrors { errors })
        }
    }
}
#
# fn main() {}
```

Each function may have a different return type and depending on the input
parameters a new result type is required. For example, adding a user
requires a new result type which contains the variant `Ok(User)`
instead of `Ok(Item)`.

The client can send a mutation request and handle the
resulting errors as shown in the following example:

```graphql
{
  mutation {
    addItem(name: "", quantity: 0) {
      ... on Item {
        name
      }
      ... on ValidationErrors {
        errors {
          field
          message
        }
      }
    }
  }
}
```

A useful side effect of this approach is to have partially successful
queries or mutations. If one resolver fails, the results of the
successful resolvers are not discarded.

### Example Input Validation (complex)

Instead of using strings to propagate errors, it is possible to use
GraphQL's type system to describe the errors more precisely.

For each fallible input variable a field in a GraphQL object is created. The
field is set if the validation for that particular field fails. You will likely want some kind of code generation to reduce repetition as the number of types required is significantly larger than
before. Each resolver function has a custom `ValidationResult` which
contains only fields provided by the function.

```rust
# extern crate juniper;
# use juniper::{graphql_object, GraphQLObject, GraphQLUnion};
#
#[derive(GraphQLObject)]
pub struct Item {
    name: String,
    quantity: i32,
}

#[derive(GraphQLObject)]
pub struct ValidationError {
    name: Option<String>,
    quantity: Option<String>,
}

#[derive(GraphQLUnion)]
pub enum GraphQLResult {
    Ok(Item),
    Err(ValidationError),
}

pub struct Mutation;

#[graphql_object]
impl Mutation {
    fn addItem(&self, name: String, quantity: i32) -> GraphQLResult {
        let mut error = ValidationError {
            name: None,
            quantity: None,
        };

        if !(10 <= name.len() && name.len() <= 100) {
            error.name = Some("between 10 and 100".into());
        }

        if !(1 <= quantity && quantity <= 10) {
            error.quantity = Some("between 1 and 10".into());
        }

        if error.name.is_none() && error.quantity.is_none() {
            GraphQLResult::Ok(Item { name, quantity })
        } else {
            GraphQLResult::Err(error)
        }
    }
}
#
# fn main() {}
```

```graphql
{
  mutation {
    addItem {
      ... on Item {
        name
      }
      ... on ValidationErrorsItem {
        name
        quantity
      }
    }
  }
}
```

Expected errors are handled directly inside the query. Additionally, all
non-critical errors are known in advance by both the server and the client.

### Example Input Validation (complex with critical error)

Our examples so far have only included non-critical errors. Providing
errors inside the GraphQL schema still allows you to return unexpected critical
errors when they occur.

In the following example, a theoretical database could fail
and would generate errors. Since it is not common for the database to
fail, the corresponding error is returned as a critical error:

```rust
# extern crate juniper;
#
use juniper::{graphql_object, graphql_value, FieldError, GraphQLObject, GraphQLUnion, ScalarValue};

#[derive(GraphQLObject)]
pub struct Item {
    name: String,
    quantity: i32,
}

#[derive(GraphQLObject)]
pub struct ValidationErrorItem {
    name: Option<String>,
    quantity: Option<String>,
}

#[derive(GraphQLUnion)]
pub enum GraphQLResult {
    Ok(Item),
    Err(ValidationErrorItem),
}

pub enum ApiError {
    Database,
}

impl<S: ScalarValue> juniper::IntoFieldError<S> for ApiError {
    fn into_field_error(self) -> FieldError<S> {
        match self {
            ApiError::Database => FieldError::new(
                "Internal database error",
                graphql_value!({
                    "type": "DATABASE"
                }),
            ),
        }
    }
}

pub struct Mutation;

#[graphql_object]
impl Mutation {
    fn addItem(&self, name: String, quantity: i32) -> Result<GraphQLResult, ApiError> {
        let mut error = ValidationErrorItem {
            name: None,
            quantity: None,
        };

        if !(10 <= name.len() && name.len() <= 100) {
            error.name = Some("between 10 and 100".into());
        }

        if !(1 <= quantity && quantity <= 10) {
            error.quantity = Some("between 1 and 10".into());
        }

        if error.name.is_none() && error.quantity.is_none() {
            Ok(GraphQLResult::Ok(Item { name, quantity }))
        } else {
            Ok(GraphQLResult::Err(error))
        }
    }
}
#
# fn main() {}
```

## Additional Material

The [Shopify API](https://shopify.dev/docs/admin-api/graphql/reference)
implements a similar approach. Their API is a good reference to
explore this approach in a real world application.

# Comparison

The first approach discussed above--where every error is a critical error defined by `FieldResult` --is easier to implement. However, the client does not know what errors may occur and must instead infer what happened from the error string. This is brittle and could change over time due to either the client or server changing. Therefore, extensive integration testing between the client and server is required to maintain the implicit contract between the two.

Encoding non-critical errors in the GraphQL schema makes the contract between the client and the server explicit. This allows the client to understand and handle these errors correctly and the server to know when changes are potentially breaking clients. However, encoding this error information into the GraphQL schema requires additional code and up-front definition of non-critical errors.


# Non-struct objects

Up until now, we've only looked at mapping structs to GraphQL objects. However,
any Rust type can be mapped into a GraphQL object. In this chapter, we'll look
at enums, but traits will work too - they don't _have_ to be mapped into GraphQL
interfaces.

Using `Result`-like enums can be a useful way of reporting e.g. validation
errors from a mutation:

```rust
# extern crate juniper;
# use juniper::{graphql_object, GraphQLObject};
# #[derive(juniper::GraphQLObject)] struct User { name: String }
#
#[derive(GraphQLObject)]
struct ValidationError {
    field: String,
    message: String,
}

# #[allow(dead_code)]
enum SignUpResult {
    Ok(User),
    Error(Vec<ValidationError>),
}

#[graphql_object]
impl SignUpResult {
    fn user(&self) -> Option<&User> {
        match *self {
            SignUpResult::Ok(ref user) => Some(user),
            SignUpResult::Error(_) => None,
        }
    }

    fn error(&self) -> Option<&Vec<ValidationError>> {
        match *self {
            SignUpResult::Ok(_) => None,
            SignUpResult::Error(ref errors) => Some(errors)
        }
    }
}
#
# fn main() {}
```

Here, we use an enum to decide whether a user's input data was valid or not, and
it could be used as the result of e.g. a sign up mutation.

While this is an example of how you could use something other than a struct to
represent a GraphQL object, it's also an example on how you could implement
error handling for "expected" errors - errors like validation errors. There are
no hard rules on how to represent errors in GraphQL, but there are
[some](https://github.com/facebook/graphql/issues/117#issuecomment-170180628)
[comments](https://github.com/graphql/graphql-js/issues/560#issuecomment-259508214)
from one of the authors of GraphQL on how they intended "hard" field errors to
be used, and how to model expected errors.
