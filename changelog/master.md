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