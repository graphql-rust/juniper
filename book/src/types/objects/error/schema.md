Schema errors
=============

[Rust]'s model of errors can be adapted for [GraphQL]. [Rust]'s panic is similar to a [field error][1] - the whole query is aborted and nothing can be extracted (except for error related information).

Not all errors require this strict handling. Recoverable or partial errors can be put into a [GraphQL schema][8], so the client can intelligently handle them.

To implement this approach, all errors must be partitioned into two classes:
- _Critical_ errors that cannot be fixed by clients (e.g. a database error).
- _Recoverable_ errors that can be fixed by clients (e.g. invalid input data).

Critical errors are returned from resolvers as [field errors][1] (from the [previous chapter](field.md)). Recoverable errors are part of a [GraphQL schema][8] and can be handled gracefully by clients. Similar to [Rust], [GraphQL] allows similar error models with [unions][9] (see ["Unions" chapter](../../unions.md)).


### Example: Simple

In this example, basic input validation is implemented with [GraphQL types][7]. [Strings][5] are used to identify the problematic [field][6] name. Errors for a particular [field][6] are also returned as a [string][5].
```rust
# extern crate juniper;
# use juniper::{GraphQLObject, GraphQLUnion, graphql_object};
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
    fn add_item(&self, name: String, quantity: i32) -> GraphQLResult {
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

Each function may have a different return type and depending on the input parameters a new result type may be required. For example, adding a `User` would require a new result type containing the variant `Ok(User)`instead of `Ok(Item)`.

> **NOTE**: In this example the returned [string][5] contains a server-side localized error message. However, it is also
possible to return a unique string identifier and have the client present a localized string to its users.

The client can send a mutation request and handle the resulting errors in the following manner:
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

> **NOTE**: A useful side effect of this approach is to have partially successful queries or mutations. If one resolver fails, the results of the successful resolvers are not discarded.


### Example: Complex

Instead of using [strings][5] to propagate errors, it is possible to use [GraphQL type system][7] to describe the errors more precisely.

For each fallible [input argument][4] we create a [field][6] in a [GraphQL object][10]. The [field][6] is set if the validation for that particular [argument][4] fails.
```rust
# extern crate juniper;
# use juniper::{GraphQLObject, GraphQLUnion, graphql_object};
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
    fn add_item(&self, name: String, quantity: i32) -> GraphQLResult {
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

> **NOTE**: We will likely want some kind of code generation to reduce repetition as the number of types required is significantly larger than before. Each resolver function has a custom `ValidationResult` which contains only [fields][6] provided by the function.

So, all the expected errors are handled directly inside the query. Additionally, all non-critical errors are known in advance by both the server and the client:
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


### Example: Complex with critical errors

Our examples so far have only included non-critical errors. Providing errors inside a [GraphQL schema][8] still allows us to return unexpected critical errors when they occur.

In the following example, a theoretical database could fail and would generate errors. Since it is not common for a database to fail, the corresponding error is returned as a [critical error][1]:
```rust
# extern crate juniper;
# use juniper::{FieldError, GraphQLObject, GraphQLUnion, ScalarValue, graphql_object, graphql_value};
#
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
            Self::Database => FieldError::new(
                "Internal database error",
                graphql_value!({"type": "DATABASE"}),
            ),
        }
    }
}

pub struct Mutation;

#[graphql_object]
impl Mutation {
    fn add_item(&self, name: String, quantity: i32) -> Result<GraphQLResult, ApiError> {
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


### Example: Shopify API

The [Shopify API] implements a similar approach. Their API is a good reference to explore this approach in a real world application.


### Example: Non-struct [objects][10]

Up until now, we've only looked at mapping [structs][20] to [GraphQL objects][10]. However, any [Rust] type can be exposed a [GraphQL object][10]. 

Using `Result`-like [enums][1] can be a useful way of reporting validation errors from a mutation:
```rust
# extern crate juniper;
# use juniper::{GraphQLObject, graphql_object};
#
#[derive(GraphQLObject)] 
struct User { 
    name: String,
}

#[derive(GraphQLObject)]
struct ValidationError {
    field: String,
    message: String,
}

enum SignUpResult {
    Ok(User),
    Error(Vec<ValidationError>),
}

#[graphql_object]
impl SignUpResult {
    fn user(&self) -> Option<&User> {
        match self {
            Self::Ok(user) => Some(user),
            Self::Error(_) => None,
        }
    }

    fn error(&self) -> Option<&[ValidationError]> {
        match self {
            Self::Ok(_) => None,
            Self::Error(errs) => Some(errs.as_slice())
        }
    }
}
#
# fn main() {}
```

Here, we use an [enum][21] to decide whether a client's input data is valid or not, and it could be used as the result of e.g. a `signUp` mutation:
```graphql
{
  mutation {
    signUp(name: "wrong") {
      user {
          name
      }
      error {
          field
          message
      }
    }
  }
}
```




[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org
[Shopify API]: https://shopify.dev/docs/admin-api/graphql/reference

[1]: https://spec.graphql.org/October2021#sec-Errors.Field-errors
[4]: https://spec.graphql.org/October2021#sec-Language.Arguments
[5]: https://spec.graphql.org/October2021#sec-String
[6]: https://spec.graphql.org/October2021#sec-Language.Fields
[7]: https://spec.graphql.org/October2021#sec-Types
[8]: https://graphql.org/learn/schema
[9]: https://spec.graphql.org/October2021#sec-Unions
[10]: https://spec.graphql.org/October2021#sec-Objects
[20]: https://doc.rust-lang.org/reference/items/structs.html
[21]: https://doc.rust-lang.org/reference/items/enumerations.html
