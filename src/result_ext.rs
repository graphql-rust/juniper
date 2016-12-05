use std::fmt;
use std::result::Result;

/**
Helper trait to produce `FieldResult`s

`FieldResult` only have strings as errors as that's what's going out
in the GraphQL response. As such, all errors must be manually
converted to strings. Importing the `ResultExt` macro and using its
only method `to_field_err` can help with that:

```rust
use std::str::FromStr;
use juniper::{FieldResult, ResultExt};

fn sample_fn(s: &str) -> FieldResult<i64> {
    i64::from_str(s).to_field_err()
}

# fn main() { assert_eq!(sample_fn("12"), Ok(12)); }
```

Alternatively, you can use the `jtry!` macro in all places you'd
normally use the regular `try!` macro:

```rust
#[macro_use] extern crate juniper;

use std::str::FromStr;

use juniper::{FieldResult, ResultExt};

fn sample_fn(s: &str) -> FieldResult<i64> {
    let value = jtry!(i64::from_str(s));

    Ok(value)
}

# fn main() { assert_eq!(sample_fn("12"), Ok(12)); }
```

 */
pub trait ResultExt<T, E: fmt::Display> {
    /// Convert the error to a string by using it's `Display` implementation
    fn to_field_err(self) -> Result<T, String>;
}

impl<T, E: fmt::Display> ResultExt<T, E> for Result<T, E> {
    fn to_field_err(self) -> Result<T, String> {
        self.map_err(|e| format!("{}", e))
    }
}

/**
Helper macro to produce `FieldResult`s.

See the documentation for the [`ResultExt`](trait.ResultExt.html)
trait.
 */
#[macro_export]
macro_rules! jtry {
    ( $e:expr ) => { try!($crate::ResultExt::to_field_err($e)) }
}
