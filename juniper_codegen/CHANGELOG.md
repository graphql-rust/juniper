`juniper_codegen` changelog
===========================

All user visible changes to `juniper_codegen` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## main

### Added

- [September 2025] GraphQL spec: ([#1347])
    - `@oneOf` input objects: ([#1354], [#1062], [#1055], [graphql/graphql-spec#825]) 
        - `enum`s support to `#[derive(GraphQLInputObject)]` macro.
    - Arguments and input object fields deprecation: ([#1348], [#864], [graphql/graphql-spec#525], [graphql/graphql-spec#805])
        - Placing `#[graphql(deprecated)]` and `#[deprecated]` attributes on struct fields in `#[derive(GraphQLInputObject)]` macro.
        - Placing `#[graphql(deprecated)]` attribute on method arguments in `#[graphql_object]` and `#[graphql_interface]` macros.
- Support of `#[graphql(rename_all = "snake_case")]` attribute. ([#1354])

[#864]: /../../issues/864
[#1055]: /../../issues/1055
[#1062]: /../../issues/1062
[#1347]: /../../issues/1347
[#1348]: /../../pull/1348
[#1354]: /../../pull/1354
[graphql/graphql-spec#525]: https://github.com/graphql/graphql-spec/pull/525
[graphql/graphql-spec#805]: https://github.com/graphql/graphql-spec/pull/805
[graphql/graphql-spec#825]: https://github.com/graphql/graphql-spec/pull/825




## [0.17.0] · 2025-09-08
[0.17.0]: /../../tree/juniper_codegen-v0.17.0/juniper_codegen

### BC Breaks

- Bumped up [MSRV] to 1.85. ([#1272], [1b1fc618])
- `#[derive(ScalarValue)]` macro: ([#1327])
    - Renamed `#[value(as_bool)]` attribute as `#[value(to_bool)]`.
    - Renamed `#[value(as_float)]` attribute as `#[value(to_float)]`.
    - Renamed `#[value(as_int)]` attribute as `#[value(to_int)]`.
    - Renamed `#[value(as_string)]` attribute as `#[value(to_string)]`.
    - Removed `#[value(into_string)]` attribute.
    - Removed `#[value(allow_missing_attributes)]` attribute.
    - `From` and `Display` implementations are not derived anymore.
- `#[derive(GraphQLScalar)]` and `#[graphql_scalar]` macros:
    - Made provided `from_input()` function to accept `ScalarValue` directly instead of `InputValue`. ([#1327])
    - Made provided `to_output()` function to return `ScalarValue` directly instead of `Value`. ([#1330])

### Added

- `#[derive(ScalarValue)]` macro:
    - Support of top-level `#[value(from_displayable_with = ...)]` attribute. ([#1324])
    - Support of top-level `#[value(from_displayable_non_static_with = ...)]` attribute. ([#1330])
- `#[derive(GraphQLScalar)]` and `#[graphql_scalar]` macros:
    - Support for specifying concrete types as input argument in provided `from_input()` function. ([#1327])
    - Support for non-`Result` return type in provided `from_input()` function. ([#1327])
    - Generating of `FromScalarValue` implementation. ([#1329])
    - Support for concrete and `impl Display` return types in provided `to_output()` function. ([#1330])
    - Generating of `ToScalarValue` implementation. ([#1330]) 

[#1272]: /../../pull/1272
[#1324]: /../../pull/1324
[#1327]: /../../pull/1327
[#1329]: /../../pull/1329
[#1330]: /../../pull/1330
[1b1fc618]: /../../commit/1b1fc61879ffdd640d741e187dc20678bf7ab295




## [0.16.0] · 2024-03-20
[0.16.0]: /../../tree/juniper_codegen-v0.16.0/juniper_codegen

### BC Breaks

- `#[graphql_object]` and `#[graphql_subscription]` expansions now preserve defined `impl` blocks "as is" and reuse defined methods in opaque way. ([#971], [#1245])
- Renamed `rename = "<policy>"` attribute argument to `rename_all = "<policy>"` (following `serde` style). ([#971])
- Redesigned `#[graphql_interface]` macro: ([#1009])
    - Removed support for `dyn` attribute argument (interface values as trait objects).
    - Removed support for `downcast` attribute argument (custom resolution into implementer types).
    - Removed support for `async` trait methods (not required anymore).
    - Removed necessity of writing `impl Trait for Type` blocks (interfaces are implemented just by matching their fields now). ([#113])
    - Forbade default implementations of non-ignored trait methods.
    - Supported coercion of additional `null`able arguments and return sub-typing on implementer.
    - Supported `rename_all = "<policy>"` attribute argument influencing all its fields and their arguments. ([#971])
    - Supported interfaces implementing other interfaces. ([#1028])
- Split `#[derive(GraphQLScalarValue)]` macro into: 
    - `#[derive(GraphQLScalar)]` for implementing GraphQL scalar: ([#1017]) 
        - Supported generic `ScalarValue`.
        - Supported structs with single named field.
        - Supported overriding resolvers with external functions, methods or modules.
        - Supported `specified_by_url` attribute argument. ([#1003], [#1000])
    - `#[derive(ScalarValue)]` for implementing `ScalarValue` trait: ([#1025])
        - Removed `Serialize` implementation (now should be provided explicitly). ([#985])
- Redesigned `#[graphql_scalar]` macro: ([#1014])
    - Changed `from_input_value()` return type from `Option` to `Result`. ([#987]) 
    - Mirrored new `#[derive(GraphQLScalar)]` macro.
    - Supported usage on type aliases in case `#[derive(GraphQLScalar)]` isn't applicable because of [orphan rules].

### Added

- `#[derive(GraphQLInterface)]` macro allowing using structs as GraphQL interfaces. ([#1026])

### Changed

- Migrated to 2 version of `syn` crate. ([#1157])

### Fixed

- All procedural macros expansion inside `macro_rules!`. ([#1054], [#1051])

[#113]: /../../issues/113
[#971]: /../../pull/971
[#985]: /../../pull/985
[#987]: /../../pull/987
[#1000]: /../../issues/1000
[#1003]: /../../pull/1003
[#1009]: /../../pull/1009
[#1014]: /../../pull/1014
[#1017]: /../../pull/1017
[#1025]: /../../pull/1025
[#1026]: /../../pull/1026
[#1028]: /../../pull/1028
[#1051]: /../../issues/1051
[#1054]: /../../pull/1054
[#1157]: /../../pull/1157
[#1245]: /../../pull/1245




[MSRV]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
[orphan rules]: https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules
[Semantic Versioning 2.0.0]: https://semver.org
[September 2025]: https://spec.graphql.org/September2025
