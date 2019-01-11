# Scalars

Scalars are the primitive types at the leaves of a GraphQL query: numbers,
strings, and booleans. You can create custom scalars to other primitive values,
but this often requires coordination with the client library intended to consume
the API you're building.

Since any value going over the wire is eventually transformed into JSON, you're
also limited in the data types you can use. Typically, you represent your custom
scalars as strings.

In Juniper, you use the `graphql_scalar!` macro to create a custom scalar. In
this example, we're representing a user ID as a string wrapped in a custom type:

```rust
use juniper::Value;

struct UserID(String);

juniper::graphql_scalar!(UserID {
    description: "An opaque identifier, represented as a string"

    resolve(&self) -> Value {
        Value::scalar(self.0.clone())
    }

    from_input_value(v: &InputValue) -> Option<UserID> {
        // If there's a parse error here, simply return None. Juniper will
        // present an error to the client.
        v.as_scalar_value::<String>().map(|s| UserID(s.to_owned()))
    }

    from_str<'a>(value: ScalarToken<'a>) -> juniper::ParseScalarResult<'a, juniper::DefaultScalarValue> {
        <String as juniper::ParseScalarValue>::from_str(value)
    }
});

# fn main() {}
```

## Built-in scalars

Juniper has built-in support for:

* `i32` as `Int`
* `f64` as `Float`
* `String` and `&str` as `String`
* `bool` as `Boolean`
* `juniper::ID` as `ID`. This type is defined [in the
  spec](http://facebook.github.io/graphql/#sec-ID) as a type that is serialized
  as a string but can be parsed from both a string and an integer.

### Non-standard scalars

Juniper has built-in support for UUIDs from the [uuid
crate](https://doc.rust-lang.org/uuid/uuid/index.html). This support is enabled
by default, but can be disabled if you want to reduce the number of dependencies
in your application.
