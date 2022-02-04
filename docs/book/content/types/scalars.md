# Scalars

Scalars are the primitive types at the leaves of a GraphQL query: numbers,
strings, and booleans. You can create custom scalars to other primitive values,
but this often requires coordination with the client library intended to consume
the API you're building.

Since any value going over the wire is eventually transformed into JSON, you're
also limited in the data types you can use.

There are two ways to define custom scalars.
* For simple scalars that just wrap a primitive type, you can use the newtype pattern with
a custom derive.
* For more advanced use cases with custom validation, you can use
the `graphql_scalar` proc macro.


## Built-in scalars

Juniper has built-in support for:

* `i32` as `Int`
* `f64` as `Float`
* `String` and `&str` as `String`
* `bool` as `Boolean`
* `juniper::ID` as `ID`. This type is defined [in the
  spec](http://facebook.github.io/graphql/#sec-ID) as a type that is serialized
  as a string but can be parsed from both a string and an integer.

Note that there is no built-in support for `i64`/`u64`, as the GraphQL spec [doesn't define any built-in scalars for `i64`/`u64` by default](https://spec.graphql.org/June2018/#sec-Int). You may wish to leverage a [custom GraphQL scalar](#custom-scalars) in your schema to support them.

**Third party types**:

Juniper has built-in support for a few additional types from common third party
crates. They are enabled via features that are on by default.

* uuid::Uuid
* chrono::DateTime
* time::{Date, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset}
* url::Url
* bson::oid::ObjectId

## Custom scalars

### `#[derive(GraphQLScalar)]`

Often, you might need a custom scalar that just wraps an existing type.

This can be done with the newtype pattern and a custom derive, similar to how
serde supports this pattern with `#[serde(transparent)]`.

```rust
# extern crate juniper;
#[derive(juniper::GraphQLScalar)]
pub struct UserId(i32);

#[derive(juniper::GraphQLObject)]
struct User {
    id: UserId,
}
#
# fn main() {}
```

That's it, you can now user `UserId` in your schema.

The macro also allows for more customization:

```rust
# extern crate juniper;
/// You can use a doc comment to specify a description.
#[derive(juniper::GraphQLScalar)]
#[graphql(
    // Overwrite the GraphQL type name.
    name = "MyUserId",
    // Specify a custom description.
    // A description in the attribute will overwrite a doc comment.
    description = "My user id description",
)]
pub struct UserId(i32);
#
# fn main() {}
```

All the methods used from newtype's field can be replaced with attributes:

#### `#[graphql(to_output_with = <fn>)]` attribute

```rust
# use juniper::{GraphQLScalar, ScalarValue, Value};
#
#[derive(GraphQLScalar)]
#[graphql(to_output_with = to_output)]
struct Incremented(i32);

/// Increments [`Incremented`] before converting into a [`Value`].
fn to_output<S: ScalarValue>(v: &Incremented) -> Value<S> {
    let inc = v.0 + 1;
    Value::from(inc)
}
# 
# fn main() {}
```

#### `#[graphql(from_input_with = <fn>, from_input_err = <type>)]` attributes

```rust
# use juniper::{DefaultScalarValue, GraphQLScalar, InputValue, ScalarValue};
#
#[derive(GraphQLScalar)]
#[graphql(scalar = DefaultScalarValue)]
#[graphql(from_input_with = Self::from_input, from_input_err = String)]
//         Unfortunately for now there is no way to infer this ^^^^^^
struct UserId(String);

impl UserId {
    /// Checks whether [`InputValue`] is `String` beginning with `id: ` and
    /// strips it.
    fn from_input(input: &InputValue) -> Result<UserId, String> {
        input.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {}", input))
            .and_then(|str| {
                str.strip_prefix("id: ")
                    .ok_or_else(|| {
                        format!(
                            "Expected `UserId` to begin with `id: `, \
                             found: {}",
                            input,
                        )
                    })
            })
            .map(|id| Self(id.to_owned()))
    }
}
#
# fn main() {}
 ```

#### `#[graphql(parse_token_with = <fn>]` or `#[graphql(parse_token(<types>)]` attributes

```rust
# use juniper::{GraphQLScalar, InputValue, ParseScalarResult, ParseScalarValue, ScalarValue, ScalarToken, Value};
#
#[derive(GraphQLScalar)]
#[graphql(
    to_output_with = to_output,
    from_input_with = from_input,
    from_input_err = String,
    parse_token_with = parse_token,
//  ^^^^^^^^^^^^^^^^ Can be replaced with `parse_token(String, i32)`
//                   which tries to parse as `String` and then as `i32`
//                   if prior fails.
)]
enum StringOrInt {
    String(String),
    Int(i32),
}

fn to_output<S: ScalarValue>(v: &StringOrInt) -> Value<S> {
    match v {
        StringOrInt::String(str) => Value::scalar(str.to_owned()),
        StringOrInt::Int(i) => Value::scalar(*i),
    }
}

fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<StringOrInt, String> {
    v.as_string_value()
        .map(|s| StringOrInt::String(s.to_owned()))
        .or_else(|| v.as_int_value().map(|i| StringOrInt::Int(i)))
        .ok_or_else(|| format!("Expected `String` or `Int`, found: {}", v))
}

fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
    <String as ParseScalarValue<S>>::from_str(value)
        .or_else(|_| <i32 as ParseScalarValue<S>>::from_str(value))
}
#
# fn main() {}
```

> __NOTE:__ As you can see, once you provide all 3 custom resolvers, there
>           is no need to follow `newtype` pattern.

#### `#[graphql(with = <module>)]` attribute

Instead of providing all custom resolvers, you can provide module with `to_output`, `from_input`, `parse_token` functions and `Error` struct or type alias.

```rust
# use juniper::{GraphQLScalar, InputValue, ParseScalarResult, ParseScalarValue, ScalarValue, ScalarToken, Value};
#
#[derive(GraphQLScalar)]
#[graphql(with = string_or_int)]
enum StringOrInt {
    String(String),
    Int(i32),
}

mod string_or_int {
    use super::*;
  
    pub(super) type Error = String;
  
    pub(super) fn to_output<S>(v: &StringOrInt) -> Value<S>
    where
        S: ScalarValue,
    {
        match v {
            StringOrInt::String(str) => Value::scalar(str.to_owned()),
            StringOrInt::Int(i) => Value::scalar(*i),
        }
    }
  
    pub(super) fn from_input<S>(v: &InputValue<S>) -> Result<StringOrInt, String>
    where
        S: ScalarValue,
    {
        v.as_string_value()
            .map(|s| StringOrInt::String(s.to_owned()))
            .or_else(|| v.as_int_value().map(|i| StringOrInt::Int(i)))
            .ok_or_else(|| format!("Expected `String` or `Int`, found: {}", v))
    }
  
    pub(super) fn parse_token<S>(value: ScalarToken<'_>) -> ParseScalarResult<'_, S>
    where
        S: ScalarValue,
    {
        <String as ParseScalarValue<S>>::from_str(value)
            .or_else(|_| <i32 as ParseScalarValue<S>>::from_str(value))
    }
}
#
# fn main() {}
```

Also, you can partially override `#[graphql(with)]` attribute with other custom scalars.

```rust
# use juniper::{GraphQLScalar, InputValue, ParseScalarResult, ScalarValue, ScalarToken, Value};
#
#[derive(GraphQLScalar)]
#[graphql(
    with = string_or_int,
    from_input_err = String,
    parse_token(String, i32)
)]
enum StringOrInt {
    String(String),
    Int(i32),
}

mod string_or_int {
    use super::*;
  
    pub(super) fn to_output<S>(v: &StringOrInt) -> Value<S>
    where
        S: ScalarValue,
    {
        match v {
            StringOrInt::String(str) => Value::scalar(str.to_owned()),
            StringOrInt::Int(i) => Value::scalar(*i),
        }
    }
  
    pub(super) fn from_input<S>(v: &InputValue<S>) -> Result<StringOrInt, String>
    where
        S: ScalarValue,
    {
        v.as_string_value()
            .map(|s| StringOrInt::String(s.to_owned()))
            .or_else(|| v.as_int_value().map(|i| StringOrInt::Int(i)))
            .ok_or_else(|| format!("Expected `String` or `Int`, found: {}", v))
    }
}
#
# fn main() {}
```

---

### `#[graphql_scalar]` attribute

For implementing custom scalars on foreign types there is `#[graphql_scalar]` attribute macro.

> __NOTE:__ To satisfy [orphan rule] you should provide local [`ScalarValue`] implementation.

```rust
# extern crate juniper;
# mod date {
#    pub struct Date;
#    impl std::str::FromStr for Date {
#        type Err = String;
#
#        fn from_str(_value: &str) -> Result<Self, Self::Err> { 
#            unimplemented!()
#        }
#    }
#
#    impl std::fmt::Display for Date {
#        fn fmt(&self, _f: &mut std::fmt::Formatter) -> std::fmt::Result {
#            unimplemented!()
#        }
#    }
# }
#
# use juniper::DefaultScalarValue as CustomScalarValue;
use juniper::{graphql_scalar, InputValue, ScalarValue, Value};

#[graphql_scalar(
    with = date_scalar, 
    parse_token = String,
    scalar = CustomScalarValue,
//           ^^^^^^^^^^^^^^^^^ Local `ScalarValue` implementation.
)]
type Date = date::Date;
//          ^^^^^^^^^^ Type from another crate.

mod date_scalar {
    use super::*;

    pub(super) type Error = String;

    pub(super) fn to_output(v: &Date) -> Value<CustomScalarValue> {
        Value::scalar(v.to_string())
    }

    pub(super) fn from_input(v: &InputValue<CustomScalarValue>) -> Result<Date, Error> {
      v.as_string_value()
          .ok_or_else(|| format!("Expected `String`, found: {}", v))
          .and_then(|s| s.parse().map_err(|e| format!("Failed to parse `Date`: {}", e)))
    }
}
#
# fn main() {}
```

[orphan rule]: https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules
[`ScalarValue`]: https://docs.rs/juniper/latest/juniper/trait.ScalarValue.html
