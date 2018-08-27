# [master] yyyy-mm-dd

## Changes

* Changed serialization of `NaiveDate` when using the optional `chronos` support.

  **Note:** while this is not a Rust breaking change, if you relied on the serialization format (perhaps by storing serialized data in a database or making asumptions in your client code written in another language) it could be a breaking change for your application.

  [#151](https://github.com/graphql-rust/juniper/pull/151)

* The `GraphQLObject`, `GraphQLInputObject`, and `GraphQLEnum` custom derives will reject
  invalid [names](http://facebook.github.io/graphql/October2016/#Name) at compile time.

  [#170](https://github.com/graphql-rust/juniper/pull/170)

* Large integers (> signed 32bit) are now deserialized as floats. Previously,
  they produced the "integer out of range" error. For languages that do not
  have distinction between integer and floating point types (including
  javascript), this means large floating point values which do not have
  fractional part could not be decoded (because they are represented without
  a decimal part `.0`).

  [#179](https://github.com/graphql-rust/juniper/pull/179)

* The `GraphQLObject`, `GraphQLInputObject`, and `GraphQLEnum` custom derives
  now parse doc strings and use them as descriptions. This behavior can be
  overridden by using an explicit GraphQL `description` annotation such as
  `#[graphql(description = "my description")]`.

  [#194](https://github.com/graphql-rust/juniper/issues/194)

* Introduced `IntoFieldError` trait to allow custom error handling
  i.e. custom result type. The error type must implement this trait resolving
  the errors into `FieldError`.

  [#40](https://github.com/graphql-rust/juniper/issues/40)
  
* `GraphQLType` and `ToInputValue` are now implemented for Arc<T>

  [#212](https://github.com/graphql-rust/juniper/pull/212)

* Error responses no longer have a *data* field, instead, error details are stored in the *extensions* field

  **Note:** while this is a breaking change, it is a necessary one to better align with the latest [GraphQL June 2018](https://facebook.github.io/graphql/June2018/#sec-Errors) specification, which defines the reserved *extensions* field for error details.  

  [#219](https://github.com/graphql-rust/juniper/pull/219)


* The `GraphQLObject` and `GraphQLInputObject` custom derives
  now support lifetime annotations.

  [#225](https://github.com/graphql-rust/juniper/issues/225)

* When using the `GraphQLObject` custom derive, fields now be omitted by annotating the field with `#[graphql(skip)]`.

  [#220](https://github.com/graphql-rust/juniper/issues/220)

* Due to newer dependencies, the oldest Rust version supported is now 1.22.0

  [#231](https://github.com/graphql-rust/juniper/pull/231)
