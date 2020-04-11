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

**Third party types**:

Juniper has built-in support for a few additional types from common third party
crates. They are enabled via features that are on by default.

* uuid::Uuid
* chrono::DateTime
* url::Url
* bson::oid::ObjectId

## newtype pattern

Often, you might need a custom scalar that just wraps an existing type. 

This can be done with the newtype pattern and a custom derive, similar to how
serde supports this pattern with `#[serde(transparent)]`.

```rust
#[derive(juniper::GraphQLScalarValue)]
pub struct UserId(i32);

#[derive(juniper::GraphQLObject)]
struct User {
    id: UserId,
}

# fn main() {}
```

That's it, you can now user `UserId` in your schema.

The macro also allows for more customization:

```rust
/// You can use a doc comment to specify a description.
#[derive(juniper::GraphQLScalarValue)]
#[graphql(
    transparent,
    // Overwrite the GraphQL type name.
    name = "MyUserId",
    // Specify a custom description.
    // A description in the attribute will overwrite a doc comment.
    description = "My user id description",
)]
pub struct UserId(i32);

# fn main() {}
```

## Custom scalars

For more complex situations where you also need custom parsing or validation, 
you can use the `graphql_scalar` proc macro.

Typically, you represent your custom scalars as strings.

The example below implements a custom scalar for a custom `Date` type.

Note: juniper already has built-in support for the `chrono::DateTime` type 
via `chrono` feature, which is enabled by default and should be used for this 
purpose.

The example below is used just for illustration.

**Note**: the example assumes that the `Date` type implements
`std::fmt::Display` and `std::str::FromStr`.


```rust
# mod date { 
#    pub struct Date; 
#    impl std::str::FromStr for Date{ 
#        type Err = String; fn from_str(_value: &str) -> Result<Self, Self::Err> { unimplemented!() }
#    }
#    // And we define how to represent date as a string.
#    impl std::fmt::Display for Date {
#        fn fmt(&self, _f: &mut std::fmt::Formatter) -> std::fmt::Result {
#            unimplemented!()
#        }
#    }
# }

use juniper::{Value, ParseScalarResult, ParseScalarValue};
use date::Date;

#[juniper::graphql_scalar(description = "Date")]
impl<S> GraphQLScalar for Date 
where
    S: ScalarValue
{
    // Define how to convert your custom scalar into a primitive type.
    fn resolve(&self) -> Value {
        Value::scalar(self.to_string())
    }

    // Define how to parse a primitive type into your custom scalar.
    fn from_input_value(v: &InputValue) -> Option<Date> {
        v.as_scalar_value()
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
    }

    // Define how to parse a string value.
    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
}

# fn main() {}
```
