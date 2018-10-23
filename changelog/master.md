# [master] yyyy-mm-dd

## Changes

- Juniper is now generic about the exact representation of scalar values. This
  allows downstream crates to add support for own scalar value representations.

   There are two use cases for this feature:
   * You want to support new scalar types not representable by the provided default
   scalar value representation like for example `i64`
   * You want to support a type from a third party crate that is not supported by juniper

  **Note:** This may need some changes in down stream code, especially if working with
  generic code. To retain the current behaviour use `DefaultScalarValue` as scalar value type

  [#251](https://github.com/graphql-rust/juniper/pull/251)
