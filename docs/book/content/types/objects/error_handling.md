# Error handling

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

## Structured errors

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
