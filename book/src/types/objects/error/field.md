Field errors
============

[Rust] provides [two ways of dealing with errors][11]:
- [`Result<T, E>`][12] for recoverable errors;
- [`panic!`][13] for unrecoverable errors.

[Juniper] does not do anything about panicking, it naturally bubbles up to the surrounding code/framework and can be dealt with there.

For recoverable errors, [Juniper] works well with the [built-in `Result` type][12]. You can use the [`?` operator][14] and things will work as you expect them to:
```rust
# extern crate juniper;
# use std::{fs::File, io::Read, path::PathBuf, str};
# use juniper::{FieldResult, graphql_object};
#
struct Example {
    filename: PathBuf,
}

#[graphql_object]
impl Example {
    fn contents(&self) -> FieldResult<String> {
        let mut file = File::open(&self.filename)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }

    fn foo() -> FieldResult<Option<String>> {
        // Some invalid bytes.
        let invalid = vec![128, 223];

        Ok(Some(str::from_utf8(&invalid)?.to_string()))
    }
}
#
# fn main() {}
```

[`FieldResult<T>`][21] is an alias for [`Result<T, FieldError>`][22], which is the [error type][1] all fallible [fields][6] must return. By using the [`?` operator][14], any type that implements the [`Display` trait][15] (which most of the error types out there do) can be automatically converted into a [`FieldError`][22].

> **TIP**: If a custom conversion into a [`FieldError`][22] is needed (to [fill up `extensions`][2], for example), the [`IntoFieldError` trait][23] should be implemented.

> **NOTE**: [`FieldError`][22]s are [GraphQL field errors][1] and are [not visible][9] in a [GraphQL schema][8] in any way.




## Error payloads, `null`, and partial errors

[Juniper]'s error behavior conforms to the [GraphQL specification][0].

When a [field][6] returns an [error][11], the [field][6]'s result is replaced by `null`, and an additional `errors` object is created at the top level of the [response][7], and the [execution][5] is resumed.

Let's run the following query against the previous example:
```graphql
{
  example {
    contents
    foo
  }
}
```

If `str::from_utf8` results in a `std::str::Utf8Error`, then the following will be returned:
```json
{
  "data": {
    "example": {
      "contents": "<Contents of the file>",
      "foo": null
    }
  },
  "errors": [{
    "message": "invalid utf-8 sequence of 2 bytes from index 0",
    "locations": [{"line": 2, "column": 4}]
  }]
}
```

> Since [`Non-Null` type][4] [fields][5] cannot be **null**, [field errors][1] are propagated to be handled by the parent [field][5]. If the parent [field][5] may be **null** then it resolves to **null**, otherwise if it is a [`Non-Null` type][4], the [field error][1] is further propagated to its parent [field][5].

For example, with the following query:
```graphql
{
  example {
    contents
  }
}
```

If the `File::open()` above results in a `std::io::ErrorKind::PermissionDenied`, the following ill be returned:
```json
{
  "data": null,
  "errors": [{
    "message": "Permission denied (os error 13)",
    "locations": [{"line": 2, "column": 4}]
  }]
}
```




## Additional information

Sometimes it's desirable to return additional structured error information to clients. This can be accomplished by implementing the [`IntoFieldError` trait][23]:
```rust
# #[macro_use] extern crate juniper;
# use juniper::{FieldError, IntoFieldError, ScalarValue, graphql_object};
#
enum CustomError {
    WhateverNotSet,
}

impl<S: ScalarValue> IntoFieldError<S> for CustomError {
    fn into_field_error(self) -> FieldError<S> {
        match self {
            Self::WhateverNotSet => FieldError::new(
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

#[graphql_object]
impl Example {
    fn whatever(&self) -> Result<bool, CustomError> {
        if let Some(value) = self.whatever {
            return Ok(value);
        }
        Err(CustomError::WhateverNotSet)
    }
}
#
# fn main() {}
```
And the specified structured error information will be included into the [error's `extensions`][2]:
```json
{
  "errors": [{
    "message": "Whatever does not exist",
    "locations": [{"line": 2, "column": 4}],
    "extensions": {
      "type": "NO_WHATEVER"
    }
  }]
}
```
> **NOTE**: This pattern is particularly useful when it comes to instrumentation of returned [field errors][1] with custom error codes or additional diagnostics (like stack traces or tracing IDs).




[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org

[0]: https://spec.graphql.org/October2021#sec-Handling-Field-Errors
[1]: https://spec.graphql.org/October2021#sec-Errors.Field-errors
[2]: https://spec.graphql.org/October2021#sel-GAPHRPZCAACCC_7Q
[4]: https://spec.graphql.org/October2021#sec-Non-Null
[5]: https://spec.graphql.org/October2021#sec-Execution
[6]: https://spec.graphql.org/October2021#sec-Language.Fields
[7]: https://spec.graphql.org/October2021#sec-Response
[8]: https://graphql.org/learn/schema
[9]: https://spec.graphql.org/October2021#sec-Introspection
[11]: https://doc.rust-lang.org/book/ch09-00-error-handling.html
[12]: https://doc.rust-lang.org/stable/std/result/enum.Result.html
[13]: https://doc.rust-lang.org/stable/std/macro.panic.html
[14]: https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html#a-shortcut-for-propagating-errors-the--operator
[15]: https://doc.rust-lang.org/stable/std/fmt/trait.Display.html
[21]: https://docs.rs/juniper/0.17.1/juniper/executor/type.FieldResult.html
[22]: https://docs.rs/juniper/0.17.1/juniper/executor/struct.FieldError.html
[23]: https://docs.rs/juniper/0.17.1/juniper/executor/trait.IntoFieldError.html
