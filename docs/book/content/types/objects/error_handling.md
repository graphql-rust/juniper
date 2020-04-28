# Error handling

Error handling in GraphQL can be done in multiple ways. In the
following two different error handling models are discussed: field
results and GraphQL schema backed errors. Each approach has its
advantages. Choosing the right error handling method depends on the
requirements of the application--investigating both approaches is
beneficial.

## Field Results

Rust
[provides](https://doc.rust-lang.org/book/second-edition/ch09-00-error-handling.html)
two ways of dealing with errors: `Result<T, E>` for recoverable errors and
`panic!` for unrecoverable errors. Juniper does not do anything about panicking;
it will bubble up to the surrounding framework and hopefully be dealt with
there.

For recoverable errors, Juniper works well with the built-in `Result` type, you
can use the `?` operator or the `try!` macro and things will generally just work
as you expect them to:

```rust
# extern crate juniper;
use std::{
    str,
    path::PathBuf,
    fs::{File},
    io::{Read},
};
use juniper::FieldResult;

struct Example {
    filename: PathBuf,
}

#[juniper::graphql_object]
impl Example {
    fn contents() -> FieldResult<String> {
        let mut file = File::open(&self.filename)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }

    fn foo() -> FieldResult<Option<String>> {
      // Some invalid bytes.
      let invalid = vec![128, 223];

      match str::from_utf8(&invalid) {
        Ok(s) => Ok(Some(s.to_string())),
        Err(e) => Err(e)?,
      }
    }
}

# fn main() {}
```

`FieldResult<T>` is an alias for `Result<T, FieldError>`, which is the error
type all fields must return. By using the `?` operator or `try!` macro, any type
that implements the `Display` trait - which are most of the error types out
there - those errors are automatically converted into `FieldError`.

When a field returns an error, the field's result is replaced by `null`, an
additional `errors` object is created at the top level of the response, and the
execution is resumed. For example, with the previous example and the following
query:

```graphql
{
  example {
    contents
    foo
  }
}
```

If `str::from_utf8` resulted in a `std::str::Utf8Error`, the following would be
returned:

!FILENAME Response for nullable field with error

```js
{
  "data": {
    "example": {
      contents: "<Contents of the file>",
      foo: null,
    }
  },
  "errors": [
    "message": "invalid utf-8 sequence of 2 bytes from index 0",
    "locations": [{ "line": 2, "column": 4 }])
  ]
}
```

If an error is returned from a non-null field, such as the
example above, the `null` value is propagated up to the first nullable parent
field, or the root `data` object if there are no nullable fields.

For example, with the following query:

```graphql
{
  example {
    contents
  }
}
```

If `File::open()` above resulted in `std::io::ErrorKind::PermissionDenied`, the
following would be returned:

!FILENAME Response for non-null field with error and no nullable parent

```js
{
  "errors": [
    "message": "Permission denied (os error 13)",
    "locations": [{ "line": 2, "column": 4 }])
  ]
}
```

### Structured errors

Sometimes it is desirable to return additional structured error information
to clients. This can be accomplished by implementing [`IntoFieldError`](https://docs.rs/juniper/latest/juniper/trait.IntoFieldError.html):

```rust
# #[macro_use] extern crate juniper;
enum CustomError {
    WhateverNotSet,
}

impl juniper::IntoFieldError for CustomError {
    fn into_field_error(self) -> juniper::FieldError {
        match self {
            CustomError::WhateverNotSet => juniper::FieldError::new(
                "Whatever does not exist",
                graphql_value!({
                    "type": "NO_WHATEVER"
                }),
            ),
        }
    }
}

struct Example {
    whatever: Option<bool>,
}

#[juniper::graphql_object]
impl Example {
    fn whatever() -> Result<bool, CustomError> {
      if let Some(value) = self.whatever {
        return Ok(value);
      }
      Err(CustomError::WhateverNotSet)
    }
}

# fn main() {}
```

The specified structured error information is included in the [`extensions`](https://facebook.github.io/graphql/June2018/#sec-Errors) key:

```js
{
  "errors": [
    "message": "Whatever does not exist",
    "locations": [{ "line": 2, "column": 4 }]),
    "extensions": {
      "type": "NO_WHATEVER"
    }
  ]
}
```

## Errors Backed by GraphQL's Schema

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
#[derive(juniper::GraphQLObject)]
pub struct Item {
    name: String,
    quantity: i32,
}

#[derive(juniper::GraphQLObject)]
pub struct ValidationError {
    field: String,
    message: String,
}

#[derive(juniper::GraphQLObject)]
pub struct ValidationErrors {
    errors: Vec<ValidationError>,
}

#[derive(juniper::GraphQLUnion)]
pub enum GraphQLResult {
    Ok(Item),
    Err(ValidationErrors),
}

pub struct Mutation;

#[juniper::graphql_object]
impl Mutation {
    fn addItem(&self, name: String, quantity: i32) -> GraphQLResult {
        let mut errors = Vec::new();

        if !(10 <= name.len() && name.len() <= 100) {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: "between 10 and 100".to_string()
            });
        }

        if !(1 <= quantity && quantity <= 10) {
            errors.push(ValidationError {
                field: "quantity".to_string(),
                message: "between 1 and 10".to_string()
            });
        }

        if errors.is_empty() {
            GraphQLResult::Ok(Item { name, quantity })
        } else {
            GraphQLResult::Err(ValidationErrors { errors })
        }
    }
}

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
#[derive(juniper::GraphQLObject)]
pub struct Item {
    name: String,
    quantity: i32,
}

#[derive(juniper::GraphQLObject)]
pub struct ValidationError {
    name: Option<String>,
    quantity: Option<String>,
}

#[derive(juniper::GraphQLUnion)]
pub enum GraphQLResult {
    Ok(Item),
    Err(ValidationError),
}

pub struct Mutation;

#[juniper::graphql_object]
impl Mutation {
    fn addItem(&self, name: String, quantity: i32) -> GraphQLResult {
        let mut error = ValidationError {
            name: None,
            quantity: None,
        };

        if !(10 <= name.len() && name.len() <= 100) {
            error.name = Some("between 10 and 100".to_string());
        }

        if !(1 <= quantity && quantity <= 10) {
            error.quantity = Some("between 1 and 10".to_string());
        }

        if error.name.is_none() && error.quantity.is_none() {
            GraphQLResult::Ok(Item { name, quantity })
        } else {
            GraphQLResult::Err(error)
        }
    }
}

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
# #[macro_use] extern crate juniper;

#[derive(juniper::GraphQLObject)]
pub struct Item {
    name: String,
    quantity: i32,
}

#[derive(juniper::GraphQLObject)]
pub struct ValidationErrorItem {
    name: Option<String>,
    quantity: Option<String>,
}

#[derive(juniper::GraphQLUnion)]
pub enum GraphQLResult {
    Ok(Item),
    Err(ValidationErrorItem),
}

pub enum ApiError {
    Database,
}

impl juniper::IntoFieldError for ApiError {
    fn into_field_error(self) -> juniper::FieldError {
        match self {
            ApiError::Database => juniper::FieldError::new(
                "Internal database error",
                graphql_value!({
                    "type": "DATABASE"
                }),
            ),
        }
    }
}

pub struct Mutation;

#[juniper::graphql_object]
impl Mutation {
    fn addItem(&self, name: String, quantity: i32) -> Result<GraphQLResult, ApiError> {
        let mut error = ValidationErrorItem {
            name: None,
            quantity: None,
        };

        if !(10 <= name.len() && name.len() <= 100) {
            error.name = Some("between 10 and 100".to_string());
        }

        if !(1 <= quantity && quantity <= 10) {
            error.quantity = Some("between 1 and 10".to_string());
        }

        if error.name.is_none() && error.quantity.is_none() {
            Ok(GraphQLResult::Ok(Item { name, quantity }))
        } else {
            Ok(GraphQLResult::Err(error))
        }
    }
}

# fn main() {}
```

## Additional Material

The [Shopify API](https://shopify.dev/docs/admin-api/graphql/reference)
implements a similar approach. Their API is a good reference to
explore this approach in a real world application.

# Comparison

The first approach discussed above--where every error is a critical error defined by `FieldResult` --is easier to implement. However, the client does not know what errors may occur and must instead infer what happened from the error string. This is brittle and could change over time due to either the client or server changing. Therefore, extensive integration testing between the client and server is required to maintain the implicit contract between the two.

Encoding non-critical errors in the GraphQL schema makes the contract between the client and the server explicit. This allows the client to understand and handle these errors correctly and the server to know when changes are potentially breaking clients. However, encoding this error information into the GraphQL schema requires additional code and up-front definition of non-critical errors.
