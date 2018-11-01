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

- The `GraphQLObject` and `GraphQLEnum` derives will mark graphql fields as
  `@deprecated` when struct fields or enum variants are marked with the
  builtin `#[deprecated]` attribute.

  The deprecation reason can be set using the `note = ...` meta item
  (e.g. `#[deprecated(note = "Replaced by betterField")]`).
  The `since` attribute is ignored.

  [#269](https://github.com/graphql-rust/juniper/pull/269)


- There is an alternative syntax for setting a field's _description_ and
  _deprecation reason_ in the `graphql_object!` and `graphql_interface!` macros.

  To __deprecate__ a graphql field:
    ```rust
    // Original syntax for setting deprecation reason
    field deprecated "Reason" my_field() -> { ... }

    // New alternative syntax for deprecation reason.
    #[deprecated(note = "Reason")]
    field my_field() -> { ... }

    // You can now also deprecate without a reason.
    #[deprecated]
    field my_field() -> { ... }
    ```

  To set the __description__ of a graphql field:
    ```rust
    // Original syntax for field descriptions
    field my_field() as "Description" -> { ... }

    // Original syntax for argument descriptions
    field my_field(
      floops: i32 as "The number of starfish to be returned. \
                      Can't be more than 100.",
    ) -> {
      ...
    }

    // New alternative syntax for field descriptions
    /// Description
    field my_field() -> { ... }

    // New alternative syntax for argument descriptions
    field my_field(
      /// The number of starfish to be returned.
      /// Can't be more than 100.
      arg: i32,
    ) -> {
      ...
    }

    // You can also use raw strings and const &'static str.
    //
    // Multiple docstrings will be collapsed into a single
    // description separated by newlines.
    /// This is my field.
    ///
    /// Make sure not to filtz the bitlet.
    /// Flitzing without a bitlet has undefined behaviour.
    ///
    #[doc = my_consts::ADDED_IN_VERSION_XYZ]
    field my_field() -> { ... }
    ```

  [#269](https://github.com/graphql-rust/juniper/pull/269)
