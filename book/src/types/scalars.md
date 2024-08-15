Scalars
=======

[GraphQL scalars][0] represent primitive leaf values in a GraphQL type system: numbers, strings, and booleans.




## Built-in

[Juniper] provides support for all the [built-in scalars][5].

| [Rust] types     | [GraphQL] scalar |
|------------------|------------------|
| `bool`           | `Boolean`        |
| `i32`            | `Int`            |
| `f64`            | `Float`          |
| `String`, `&str` | `String`         |
| `juniper::ID`    | [`ID`]           |

> **NOTE**: [`ID`] scalar is [defined in the GraphQL spec][`ID`] as a type that is serialized as a string, but can be parsed from both a string and an integer.

> **TIP**: There is no built-in support for `i64`, `u64`, or other [Rust] integer types, as the [GraphQL spec doesn't define any built-in scalars for them][1] by default. Instead, to be supported, they should be defined as [custom scalars](#custom) in a [GraphQL schema][schema].




## Custom

We can create [custom scalars][2] for other primitive values, but they are still [limited in the data types for representation][1], and only introduce additional semantic meaning. This, also, often requires coordination with the client library, intended to consume the API we're building.

[Custom scalars][2] can be defined in [Juniper] by using either [`#[derive(GraphQLScalar)]`][8] or [`#[graphql_scalar]`][9] attributes, which do work pretty much the same way (except, [`#[derive(GraphQLScalar)]`][8] cannot be used on [type aliases][4]).


### Transparent delegation

Quite often, we want to create a [custom GraphQL scalar][2] type by just wrapping an existing one, inheriting all its behavior. In [Rust], this is often called as ["newtype pattern"][3]. This may be achieved by providing a `#[graphql(transparent)]` attribute to the definition:
```rust
# extern crate juniper;
# use juniper::{graphql_scalar, GraphQLScalar};
#
#[derive(GraphQLScalar)]
#[graphql(transparent)]
pub struct UserId(i32);

// Using `#[graphql_scalar]` attribute here makes no difference, and is fully
// interchangeable with `#[derive(GraphQLScalar)]`. It's only up to the 
// personal preference - which one to use.
#[graphql_scalar]
#[graphql(transparent)]
pub struct MessageId {
  value: i32,
}
#
# fn main() {}
```
That's it, now the `UserId` and `MessageId` [scalars][0] can be used in [GraphQL schema][schema].

We may also customize the definition, to provide more information about our [custom scalar][2] in [GraphQL schema][schema]:
```rust
# extern crate juniper;
# use juniper::GraphQLScalar;
#
/// You can use a Rust doc comment to specify a description in GraphQL schema.
#[derive(GraphQLScalar)]
#[graphql(
    transparent,
    // Overwrite the name of this type in the GraphQL schema.
    name = "MyUserId",
    // Specifying a type description via attribute takes precedence over the
    // Rust doc comment, which allows to separate Rust API docs from GraphQL 
    // schema descriptions, if required.
    description = "Actual description.",
    // Optional specification URL.
    specified_by_url = "https://tools.ietf.org/html/rfc4122",
)]
pub struct UserId(String);
#
# fn main() {}
```


### Resolving

In case we need to customize [resolving][7] of a [custom GraphQL scalar][2] value (change the way it gets executed), the `#[graphql(to_output_with = <fn path>)]` attribute is the way to do so:
```rust
# extern crate juniper;
# use juniper::{GraphQLScalar, ScalarValue, Value};
#
#[derive(GraphQLScalar)]
#[graphql(to_output_with = to_output, transparent)]
struct Incremented(i32);

/// Increments [`Incremented`] before converting into a [`Value`].
fn to_output<S: ScalarValue>(v: &Incremented) -> Value<S> {
    let inc = v.0 + 1;
    Value::from(inc)
}
#
# fn main() {}
```


### Input value parsing

Customization of a [custom GraphQL scalar][2] value parsing is possible via `#[graphql(from_input_with = <fn path>)]` attribute:
```rust
# extern crate juniper;
# use juniper::{GraphQLScalar, InputValue, ScalarValue};
#
#[derive(GraphQLScalar)]
#[graphql(from_input_with = Self::from_input, transparent)]
struct UserId(String);

impl UserId {
    /// Checks whether the [`InputValue`] is a [`String`] beginning with `id: ` 
    /// and strips it.
    fn from_input<S>(input: &InputValue<S>) -> Result<Self, String> 
    where
        S: ScalarValue
    {
        input.as_string_value()
            .ok_or_else(|| format!("Expected `String`, found: {input}"))
            .and_then(|str| {
                str.strip_prefix("id: ")
                    .ok_or_else(|| {
                        format!(
                            "Expected `UserId` to begin with `id: `, \
                             found: {input}",
                        )
                    })
            })
            .map(|id| Self(id.to_owned()))
    }
}
#
# fn main() {}
```


### Token parsing

Customization of which tokens a [custom GraphQL scalar][0] type should be parsed from, is possible via `#[graphql(parse_token_with = <fn path>)]` or `#[graphql(parse_token(<types>)]` attributes:
```rust
# extern crate juniper;
# use juniper::{
#     GraphQLScalar, InputValue, ParseScalarResult, ParseScalarValue,
#     ScalarValue, ScalarToken, Value,
# };
#
#[derive(GraphQLScalar)]
#[graphql(
    to_output_with = to_output,
    from_input_with = from_input,
    parse_token_with = parse_token,
)]
//  ^^^^^^^^^^^^^^^^ Can be replaced with `parse_token(String, i32)`, which
//                   tries to parse as `String` first, and then as `i32` if
//                   prior fails.
enum StringOrInt {
    String(String),
    Int(i32),
}

fn to_output<S: ScalarValue>(v: &StringOrInt) -> Value<S> {
    match v {
        StringOrInt::String(s) => Value::scalar(s.to_owned()),
        StringOrInt::Int(i) => Value::scalar(*i),
    }
}

fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<StringOrInt, String> {
    v.as_string_value()
        .map(|s| StringOrInt::String(s.into()))
        .or_else(|| v.as_int_value().map(StringOrInt::Int))
        .ok_or_else(|| format!("Expected `String` or `Int`, found: {v}"))
}

fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<S> {
    <String as ParseScalarValue<S>>::from_str(value)
        .or_else(|_| <i32 as ParseScalarValue<S>>::from_str(value))
}
#
# fn main() {}
```
> **NOTE**: Once we provide all 3 custom functions, there is no sense to follow [newtype pattern][3] anymore, as nothing left to inherit.


### Full behavior

Instead of providing all custom functions separately, it's possible to provide a module holding the appropriate `to_output()`, `from_input()` and `parse_token()` functions via `#[graphql(with = <module path>)]` attribute:
```rust
# extern crate juniper;
# use juniper::{
#     GraphQLScalar, InputValue, ParseScalarResult, ParseScalarValue,
#     ScalarValue, ScalarToken, Value,
# };
#
#[derive(GraphQLScalar)]
#[graphql(with = string_or_int)]
enum StringOrInt {
    String(String),
    Int(i32),
}

mod string_or_int {
    use super::*;

    pub(super) fn to_output<S: ScalarValue>(v: &StringOrInt) -> Value<S> {
        match v {
            StringOrInt::String(s) => Value::scalar(s.to_owned()),
            StringOrInt::Int(i) => Value::scalar(*i),
        }
    }

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<StringOrInt, String> {
        v.as_string_value()
            .map(|s| StringOrInt::String(s.into()))
            .or_else(|| v.as_int_value().map(StringOrInt::Int))
            .ok_or_else(|| format!("Expected `String` or `Int`, found: {v}"))
    }

    pub(super) fn parse_token<S: ScalarValue>(t: ScalarToken<'_>) -> ParseScalarResult<S> {
        <String as ParseScalarValue<S>>::from_str(t)
            .or_else(|_| <i32 as ParseScalarValue<S>>::from_str(t))
    }
}
#
# fn main() {}
```

A regular `impl` block is also suitable for that:
```rust
# extern crate juniper;
# use juniper::{
#     GraphQLScalar, InputValue, ParseScalarResult, ParseScalarValue,
#     ScalarValue, ScalarToken, Value,
# };
#
#[derive(GraphQLScalar)]
// #[graphql(with = Self)] <- default behaviour, so can be omitted
enum StringOrInt {
    String(String),
    Int(i32),
}

impl StringOrInt {
    fn to_output<S: ScalarValue>(&self) -> Value<S> {
        match self {
            Self::String(s) => Value::scalar(s.to_owned()),
            Self::Int(i) => Value::scalar(*i),
        }
    }

    fn from_input<S>(v: &InputValue<S>) -> Result<Self, String>
    where
        S: ScalarValue
    {
        v.as_string_value()
            .map(|s| Self::String(s.into()))
            .or_else(|| v.as_int_value().map(Self::Int))
            .ok_or_else(|| format!("Expected `String` or `Int`, found: {v}"))
    }

    fn parse_token<S>(value: ScalarToken<'_>) -> ParseScalarResult<S>
    where
        S: ScalarValue
    {
        <String as ParseScalarValue<S>>::from_str(value)
            .or_else(|_| <i32 as ParseScalarValue<S>>::from_str(value))
    }
}
#
# fn main() {}
```

At the same time, any custom function still may be specified separately, if required:
```rust
# extern crate juniper;
# use juniper::{
#     GraphQLScalar, InputValue, ParseScalarResult, ScalarValue,
#     ScalarToken, Value
# };
#
#[derive(GraphQLScalar)]
#[graphql(
    with = string_or_int,
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
            StringOrInt::String(s) => Value::scalar(s.to_owned()),
            StringOrInt::Int(i) => Value::scalar(*i),
        }
    }

    pub(super) fn from_input<S>(v: &InputValue<S>) -> Result<StringOrInt, String>
    where
        S: ScalarValue,
    {
        v.as_string_value()
            .map(|s| StringOrInt::String(s.into()))
            .or_else(|| v.as_int_value().map(StringOrInt::Int))
            .ok_or_else(|| format!("Expected `String` or `Int`, found: {v}"))
    }

    // No need in `parse_token()` function.
}
#
# fn main() {}
```

> **TIP**: See more available features in the API docs of the [`#[derive(GraphQLScalar)]`][8] and [`#[graphql_scalar]`][9] attributes.




## Foreign

For implementing [custom scalars][2] on foreign types there is [`#[graphql_scalar]`][9] attribute.

> **NOTE**: To satisfy [orphan rules], we should provide a local [`ScalarValue`] implementation.

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
    parse_token(String),
    scalar = CustomScalarValue,
)]
//           ^^^^^^^^^^^^^^^^^ local `ScalarValue` implementation
type Date = date::Date;
//          ^^^^^^^^^^ type from another crate

mod date_scalar {
    use super::*;
  
    pub(super) fn to_output(v: &Date) -> Value<CustomScalarValue> {
        Value::scalar(v.to_string())
    }

    pub(super) fn from_input(v: &InputValue<CustomScalarValue>) -> Result<Date, String> {
      v.as_string_value()
          .ok_or_else(|| format!("Expected `String`, found: {v}"))
          .and_then(|s| s.parse().map_err(|e| format!("Failed to parse `Date`: {e}")))
    }
}
#
# fn main() {}
```


### Supported out-of-the-box

[Juniper] provides out-of-the-box [GraphQL scalar][0] implementations for some very common [Rust] crates. The types from these crates will be usable in your schemas automatically after enabling the correspondent self-titled [Cargo feature].

| [Rust] type                 | [GraphQL] scalar | [Cargo feature]  |
|-----------------------------|------------------|------------------|
| [`BigDecimal`]              | `BigDecimal`     | [`bigdecimal`]   |
| [`bson::oid::ObjectId`]     | `ObjectId`       | [`bson`]         |
| [`bson::DateTime`]          | `UtcDateTime`    | [`bson`]         |
| [`chrono::NaiveDate`]       | [`Date`]         | [`chrono`]       |
| [`chrono::NaiveTime`]       | [`LocalTime`]    | [`chrono`]       |
| [`chrono::NaiveDateTime`]   | `LocalDateTime`  | [`chrono`]       |
| [`chrono::DateTime`]        | [`DateTime`]     | [`chrono`]       |
| [`chrono_tz::Tz`]           | `TimeZone`       | [`chrono-tz`]    |
| [`Decimal`]                 | `Decimal`        | [`rust_decimal`] |
| [`jiff::civil::Date`]       | [`LocalDate`]    | [`jiff`]         |
| [`jiff::civil::Time`]       | [`LocalTime`]    | [`jiff`]         |
| [`jiff::civil::DateTime`]   | `LocalDateTime`  | [`jiff`]         |
| [`jiff::Timestamp`]         | [`DateTime`]     | [`jiff`]         |
| [`jiff::Span`]              | [`Duration`]     | [`jiff`]         |
| [`jiff::Zoned`]             | `ZonedDateTime`  | `jiff-tz`        |
| [`time::Date`]              | [`Date`]         | [`time`]         |
| [`time::Time`]              | [`LocalTime`]    | [`time`]         |
| [`time::PrimitiveDateTime`] | `LocalDateTime`  | [`time`]         |
| [`time::OffsetDateTime`]    | [`DateTime`]     | [`time`]         |
| [`time::UtcOffset`]         | [`UtcOffset`]    | [`time`]         |
| [`Url`]                     | `Url`            | [`url`]          |
| [`Uuid`]                    | `Uuid`           | [`uuid`]         |




[`bigdecimal`]: https://docs.rs/bigdecimal
[`BigDecimal`]: https://docs.rs/bigdecimal/latest/bigdecimal/struct.BigDecimal.html
[`bson`]: https://docs.rs/bson
[`bson::DateTime`]: https://docs.rs/bson/latest/bson/struct.DateTime.html
[`bson::oid::ObjectId`]: https://docs.rs/bson/latest/bson/oid/struct.ObjectId.html
[`chrono`]: https://docs.rs/chrono
[`chrono::DateTime`]: https://docs.rs/chrono/latest/chrono/struct.DateTime.html
[`chrono::NaiveDate`]: https://docs.rs/chrono/latest/chrono/naive/struct.NaiveDate.html
[`chrono::NaiveDateTime`]: https://docs.rs/chrono/latest/chrono/naive/struct.NaiveDateTime.html
[`chrono::NaiveTime`]: https://docs.rs/chrono/latest/chrono/naive/struct.NaiveTime.html
[`chrono-tz`]: https://docs.rs/chrono-tz
[`chrono_tz::Tz`]: https://docs.rs/chrono-tz/latest/chrono_tz/enum.Tz.html
[`Date`]: https://graphql-scalars.dev/docs/scalars/date
[`DateTime`]: https://graphql-scalars.dev/docs/scalars/date-time
[`Decimal`]: https://docs.rs/rust_decimal/latest/rust_decimal/struct.Decimal.html
[`Duration`]: https://graphql-scalars.dev/docs/scalars/duration
[`ID`]: https://spec.graphql.org/October2021#sec-ID
[`jiff`]: https://docs.rs/jiff
[`jiff::civil::Date`]: https://docs.rs/jiff/latest/jiff/civil/struct.Date.html
[`jiff::civil::DateTime`]: https://docs.rs/jiff/latest/jiff/civil/struct.DateTime.html
[`jiff::civil::Time`]: https://docs.rs/jiff/latest/jiff/civil/struct.Time.html
[`jiff::Span`]: https://docs.rs/jiff/latest/jiff/struct.Span.html
[`jiff::Timestamp`]: https://docs.rs/jiff/latest/jiff/struct.Timestamp.html
[`jiff::Zoned`]: https://docs.rs/jiff/latest/jiff/struct.Zoned.html
[`LocalDate`]: https://graphql-scalars.dev/docs/scalars/local-date
[`LocalTime`]: https://graphql-scalars.dev/docs/scalars/local-time
[`rust_decimal`]: https://docs.rs/rust_decimal
[`ScalarValue`]: https://docs.rs/juniper/0.16.1/juniper/trait.ScalarValue.html
[`serde`]: https://docs.rs/serde
[`time`]: https://docs.rs/time
[`time::Date`]: https://docs.rs/time/latest/time/struct.Date.html
[`time::PrimitiveDateTime`]: https://docs.rs/time/latest/time/struct.PrimitiveDateTime.html
[`time::Time`]: https://docs.rs/time/latest/time/struct.Time.html
[`time::UtcOffset`]: https://docs.rs/time/latest/time/struct.UtcOffset.html
[`time::OffsetDateTime`]: https://docs.rs/time/latest/time/struct.OffsetDateTime.html
[`url`]: https://docs.rs/url
[`Url`]: https://docs.rs/url/latest/url/struct.Url.html
[`UtcOffset`]: https://graphql-scalars.dev/docs/scalars/utc-offset
[`uuid`]: https://docs.rs/uuid
[`Uuid`]: https://docs.rs/uuid/latest/uuid/struct.Uuid.html
[Cargo feature]: https://doc.rust-lang.org/cargo/reference/features.html
[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[orphan rules]: https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules
[Rust]: https://www.rust-lang.org
[schema]: https://graphql.org/learn/schema

[0]: https://spec.graphql.org/October2021#sec-Scalars
[1]: https://spec.graphql.org/October2021#sel-FAHXJDCAACKB1qb
[2]: https://spec.graphql.org/October2021#sec-Scalars.Custom-Scalars
[3]: https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html
[4]: https://doc.rust-lang.org/reference/items/type-aliases.html
[5]: https://spec.graphql.org/October2021/#sec-Scalars.Built-in-Scalars
[6]: https://serde.rs/container-attrs.html#transparent
[7]: https://spec.graphql.org/October2021#sec-Value-Resolution
[8]: https://docs.rs/juniper/0.16.1/juniper/derive.GraphQLScalar.html
[9]: https://docs.rs/juniper/0.16.1/juniper/attr.graphql_scalar.html
