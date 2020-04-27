# Error handling

Error handling in GraphQL can be done in multiple ways. In the
following two different error handling models are discussed: field
results and GraphQL schema backed errors. Each approach has its
advantages. Choosing the right error handling method depends on the
requirements of the application. Investigating in both approaches is
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
similar to a `FieldError`. The whole query is aborted and nothing can
be extracted (except for error related information). Not all errors
require this strict handling. Recoverable or partial errors can be put
into the GraphQL scheme. This way the client knows what can usually
happen. To implement this approach all errors must be partitioned into
the two error classes. The difference to the model in Rust is that
critical means, e.g., database error. On the other hand, recoverable
errors can be fixed by the user itself. If the user changes the input,
then the error might get away. The prime example for recoverable
errors is input validation.

The error class specifies where the error is seen. Critical errors are
returned as `FieldErrors` (from the previous section). Non-critical
errors are part of the GraphQL scheme. Similar to Rust, GraphQL allows
to have a similar error model with unions (see Unions).

### Example Input Validation (simple)

In this example, a basic input validation is implemented with GraphQL
types. Strings are used to identify the problematic field name. Errors
for a particular field are also returned as a string. In this example,
the string contains a localized error message. However, it is also
possible to return a unique string identifier.

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
            GraphQLResult::Err(errors)
        }
    }
}

```

Each function may have a different return type. Depending on the input
parameters a new result type is required. For example, adding a user
require a new result type which contains the variant `Ok(User)`
instead of `Ok(Item)`.

Finally it is possible to send a mutation request and handle the
result. The following example query shows how to handle the result.

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
paths. Therefore, a if one of multiple paths fails, the result of the
successful paths are not discarded.

### Example Input Validation (complex)

Instead of using strings do propagated error, it is possible to use
GraphQL's type system to describe the errors more precisely. For each
failable input variable a field in a GraphQL object is created. The
field is set if the validation for that particular field fails. Notice
that some kind of code generation reduces the unnecessary work. The
amount of types which are required is significant larger than
before. Each functions has their custom `ValidationResult` which
contains only fields provided by the function.

```rust
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

pub struct Mutation;

#[juniper::graphql_object]
impl Mutation {
    fn addItem(&self, name: String, quantity: i32) -> GraphQLResult {
        let mut error = ValidationErrorItem {
            name: None,
            quantity: None,
        };

        if !(10 <= name.len() && name.len() <= 100) {
            error.name(Some("between 10 and 100".to_string()));
        }

        if !(1 <= quantity && quantity <= 10) {
            error.quantity(Some("between 1 and 10".to_string()));
        }

        if error.name.is_none() && error.field.is_none() {
            GraphQLResult::Ok(Item { name, quantity })
        } else {
            GraphQLResult::Err(errors)
        }
    }
}

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

Expected errors are handled directly inside the query. Even more, all
non-critical errors are known in advance.

### Example Input Validation (complex with critical error)

So far only non-critical errors occurred in the examples. Providing
errors inside the GraphQL schema still allows to return critical
errors. In the following example, a theoretical database could fail
and would generate errors. Since it is not common for the database to
fail, the corresponding error is returned as a critical error.

```rust
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
            error.name(Some("between 10 and 100".to_string()));
        }

        if !(1 <= quantity && quantity <= 10) {
            error.quantity(Some("between 1 and 10".to_string()));
        }

        if error.name.is_none() && error.field.is_none() {
            Ok(GraphQLResult::Ok(Item { name, quantity }))
        } else {
            Ok(GraphQLResult::Err(errors))
        }
    }
}

```

## Additional Material

The [Shopify API](https://shopify.dev/docs/admin-api/graphql/reference)
implements a similar approach. Their API is a good reference to
explore this approach in a real world application.

# Comparison

The first discussed approach is easier to implement. However, the
errors must be matched with strings on the frontend. The frontend does
not know which errors can occur. Testing this can be difficult. It is
very likely to get something wrong. Therefore, extensive testing
between the front and backend is required.

Instead a GraphQL's type is expose specifying what kind of errors are
likely occurring. Allowing the frontend to handle these error
correctly. However, encoding these information into the GraphQL schema
requires additional work.

If your not convinced yet, then consider the following example. An
application is available in different spoken languages. Localization
should be handled on the frontend exclusively. The frontend requires
detailed information about the error. For example, instead of return
"string to short" in different language, the expected limits of that
string (min length, max length) is returned. The frontend uses these
information to build an appropriate message. Enforcing this requires
are large amount of types, but hopefully increase the quality of the
software.
