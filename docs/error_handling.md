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
# #[macro_use] extern crate juniper;
use juniper::FieldResult;
use std::path::PathBuf;
use std::fs::File;
use std::io::Read;

struct Example {
    filename: PathBuf,
}

graphql_object!(Example: () |&self| {
    field contents() -> FieldResult<String> {
        let mut file = File::open(&self.filename)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }
});

# fn main() {}
```

`FieldResult<T>` is an alias for `Result<T, FieldError>`, which is the error
type all fields must return. By using the `?` operator or `try!` macro, any type
that implements the `Display` trait - which are most of the error types out
there - those errors are automatically converted into `FieldError`.

When a field returns an error, the field's result is replaced by `null`, an
additional `errors` object is created at the top level of the response, and the
execution is resumed. If an error is returned from a non-null field, such as the
example above, the `null` value is propagated up to the first nullable parent
field, or the root `data` object if there are no nullable fields.
