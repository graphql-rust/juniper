`juniper` changelog
===================

All user visible changes to `juniper` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.16.1] · 2024-04-04
[0.16.1]: /../../tree/juniper-v0.16.1/juniper

[Diff](/../../compare/juniper-v0.16.0...juniper-v0.16.1) | [Milestone](/../../milestone/6)

### Changed

- Updated [GraphiQL] to 3.1.2 version. ([#1251])

[#1251]: /../../pull/1251




## [0.16.0] · 2024-03-20
[0.16.0]: /../../tree/juniper-v0.16.0/juniper

[Diff](/../../compare/juniper-v0.15.12...juniper-v0.16.0) | [Milestone](/../../milestone/4)

### BC Breaks

- [October 2021] GraphQL spec: ([#1000])
    - Forbade [`__typename` field on `subscription` operations](https://spec.graphql.org/October2021#note-bc213). ([#1001])
    - Supported `isRepeatable` field on directives. ([#1003])
    - Supported `__Schema.description`, `__Type.specifiedByURL` and `__Directive.isRepeatable` fields in introspection. ([#1003])
    - Supported directives on variables definitions. ([#1005])
- Replaced `Visitor` associated type with `DeserializeOwned` requirement in `ScalarValue` trait. ([#985])
- `#[graphql_object]` and `#[graphql_subscription]` expansions now preserve defined `impl` blocks "as is" and reuse defined methods in opaque way. ([#971])
- Renamed `rename = "<policy>"` attribute argument to `rename_all = "<policy>"` (following `serde` style). ([#971])
- Upgraded [`bson` crate] integration to [2.0 version](https://github.com/mongodb/bson-rust/releases/tag/v2.0.0). ([#979])
- Upgraded [`uuid` crate] integration to [1.0 version](https://github.com/uuid-rs/uuid/releases/tag/1.0.0). ([#1057])
- Upgraded [`chrono-tz` crate] integration to [0.8 version](https://github.com/chronotope/chrono-tz/blob/ea628d3131b4a659acb42dbac885cfd08a2e5de9/CHANGELOG.md#080). ([#1119])
- Upgraded [`bigdecimal` crate] integration to 0.4 version. ([#1176])
- Made `FromInputValue` trait methods fallible to allow post-validation. ([#987])
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
- Renamed `ScalarValue::as_boolean` method to `ScalarValue::as_bool`. ([#1025])
- Reworked [`chrono` crate] integration GraphQL scalars according to [graphql-scalars.dev] specs: ([#1010])
    - Disabled `chrono` [Cargo feature] by default.
    - Removed `scalar-naivetime` [Cargo feature].
- Removed lifetime parameter from `ParseError`, `GraphlQLError`, `GraphQLBatchRequest` and `GraphQLRequest`. ([#1081], [#528])
- Upgraded [GraphiQL] to 3.1.1 version (requires new [`graphql-transport-ws` GraphQL over WebSocket Protocol] integration on server, see `juniper_warp/examples/subscription.rs`). ([#1188], [#1193], [#1246])
- Abstracted `Spanning::start` and `Spanning::end` fields into separate struct `Span`. ([#1207], [#1208])
- Removed `graphql-parser-integration` and `graphql-parser` [Cargo feature]s by merging them into `schema-language` [Cargo feature]. ([#1237])
- Renamed `RootNode::as_schema_language()` method as `RootNode::as_sdl()`. ([#1237]) 
- Renamed `RootNode::as_parser_document()` method as `RootNode::as_document()`. ([#1237])
- Reworked look-ahead machinery: ([#1212])
    - Turned from eagerly-evaluated into lazy-evaluated:
        - Made `LookAheadValue::List` to contain new iterable `LookAheadList` type.
        - Made `LookAheadValue::Object` to contain new iterable `LookAheadObject` type.
    - Removed `LookAheadMethods` trait and redundant `ConcreteLookAheadSelection` type, making all APIs accessible as inherent methods on `LookAheadSelection` and `LookAheadChildren` decoupled types:
        - Moved `LookAheadMethods::child_names()` to `LookAheadChildren::names()`.
        - Moved `LookAheadMethods::has_children()` to `LookAheadChildren::is_empty()`.
        - Moved `LookAheadMethods::select_child()` to `LookAheadChildren::select()`.
        - Moved `LookAheadSelection::for_explicit_type()` to `LookAheadSelection::children_for_explicit_type()`.
        - Made `LookAheadSelection::arguments()` returning iterator over `LookAheadArgument`.
        - Made `LookAheadSelection::children()` returning `LookAheadChildren`.
    - Added `Span` to `Arguments` and `LookAheadArguments`. ([#1206], [#1209])
- Disabled `bson`, `url`, `uuid` and `schema-language` [Cargo feature]s by default. ([#1230])

### Added

- Usage of Rust arrays as GraphQL lists. ([#966], [#918])
- `From` implementations for `InputValue` mirroring the ones for `Value` and better support for `Option` handling. ([#996])
- `null` in addition to `None` for creating `Value::Null` in `graphql_value!` macro (following `serde_json::json!` style). ([#996])
- `graphql_input_value!` and `graphql_vars!` macros. ([#996])
- [`time` crate] integration behind `time` [Cargo feature]. ([#1006])
- `#[derive(GraphQLInterface)]` macro allowing using structs as GraphQL interfaces. ([#1026])
- [`bigdecimal` crate] integration behind `bigdecimal` [Cargo feature]. ([#1060])
- [`rust_decimal` crate] integration behind `rust_decimal` [Cargo feature]. ([#1060])
- `js` [Cargo feature] enabling `js-sys` and `wasm-bindgen` support for `wasm32-unknown-unknown` target. ([#1118], [#1147])
- `LookAheadMethods::applies_for()` method. ([#1138], [#1145])
- `LookAheadMethods::field_original_name()` and `LookAheadMethods::field_alias()` methods. ([#1199])
- [`anyhow` crate] integration behind `anyhow` and `backtrace` [Cargo feature]s. ([#1215], [#988])
- `RootNode::disable_introspection()` applying additional `validation::rules::disable_introspection`, and `RootNode::enable_introspection()` reverting it. ([#1227], [#456])
- `Clone` and `PartialEq` implementations for `GraphQLResponse`. ([#1228], [#103])

### Changed

- Made `GraphQLRequest` fields public. ([#750])
- Relaxed [object safety] requirement for `GraphQLValue` and `GraphQLValueAsync` traits. ([ba1ed85b])
- Updated [GraphQL Playground] to 1.7.28 version. ([#1190])
- Improve validation errors for input values. ([#811], [#693])

## Fixed

- Unsupported spreading GraphQL interface fragments on unions and other interfaces. ([#965], [#798])
- Unsupported expressions in `graphql_value!` macro. ([#996], [#503])
- Incorrect GraphQL list coercion rules: `null` cannot be coerced to an `[Int!]!` or `[Int]!`. ([#1004])
- All procedural macros expansion inside `macro_rules!`. ([#1054], [#1051])
- Incorrect input value coercion with defaults. ([#1080], [#1073])
- Incorrect error when explicit `null` provided for `null`able list input parameter. ([#1086], [#1085])
- Stack overflow on nested GraphQL fragments. ([CVE-2022-31173])
- Unstable definitions order in schema generated by `RootNode::as_sdl()`. ([#1237], [#1134])
- Unstable definitions order in schema generated by `introspect()` or other introspection queries. ([#1239], [#1134])

[#103]: /../../issues/103
[#113]: /../../issues/113
[#456]: /../../issues/456
[#503]: /../../issues/503
[#528]: /../../issues/528
[#693]: /../../issues/693
[#750]: /../../issues/750
[#798]: /../../issues/798
[#811]: /../../pull/811
[#918]: /../../issues/918
[#965]: /../../pull/965
[#966]: /../../pull/966
[#971]: /../../pull/971
[#979]: /../../pull/979
[#985]: /../../pull/985
[#987]: /../../pull/987
[#988]: /../../issues/988
[#996]: /../../pull/996
[#1000]: /../../issues/1000
[#1001]: /../../pull/1001
[#1003]: /../../pull/1003
[#1004]: /../../pull/1004
[#1005]: /../../pull/1005
[#1006]: /../../pull/1006
[#1009]: /../../pull/1009
[#1010]: /../../pull/1010
[#1014]: /../../pull/1014
[#1017]: /../../pull/1017
[#1025]: /../../pull/1025
[#1026]: /../../pull/1026
[#1028]: /../../pull/1028
[#1051]: /../../issues/1051
[#1054]: /../../pull/1054
[#1057]: /../../pull/1057
[#1060]: /../../pull/1060
[#1073]: /../../issues/1073
[#1080]: /../../pull/1080
[#1081]: /../../pull/1081
[#1085]: /../../issues/1085
[#1086]: /../../pull/1086
[#1118]: /../../issues/1118
[#1119]: /../../pull/1119
[#1134]: /../../issues/1134
[#1138]: /../../issues/1138
[#1145]: /../../pull/1145
[#1147]: /../../pull/1147
[#1176]: /../../pull/1176
[#1188]: /../../pull/1188
[#1190]: /../../pull/1190
[#1193]: /../../pull/1193
[#1199]: /../../pull/1199
[#1206]: /../../pull/1206
[#1207]: /../../pull/1207
[#1208]: /../../pull/1208
[#1209]: /../../pull/1209
[#1212]: /../../pull/1212
[#1215]: /../../pull/1215
[#1227]: /../../pull/1227
[#1228]: /../../pull/1228
[#1230]: /../../pull/1230
[#1237]: /../../pull/1237
[#1239]: /../../pull/1239
[#1246]: /../../pull/1246
[ba1ed85b]: /../../commit/ba1ed85b3c3dd77fbae7baf6bc4e693321a94083
[CVE-2022-31173]: /../../security/advisories/GHSA-4rx6-g5vg-5f3j




## Previous releases

See [old CHANGELOG](/../../blob/juniper-v0.15.12/juniper/CHANGELOG.md).




[`anyhow` crate]: https://docs.rs/anyhow
[`bigdecimal` crate]: https://docs.rs/bigdecimal
[`bson` crate]: https://docs.rs/bson
[`chrono` crate]: https://docs.rs/chrono
[`chrono-tz` crate]: https://docs.rs/chrono-tz
[`time` crate]: https://docs.rs/time
[Cargo feature]: https://doc.rust-lang.org/cargo/reference/features.html
[`graphql-transport-ws` GraphQL over WebSocket Protocol]: https://github.com/enisdenjo/graphql-ws/v5.14.0/PROTOCOL.md 
[GraphiQL]: https://github.com/graphql/graphiql
[GraphQL Playground]: https://github.com/prisma/graphql-playground
[graphql-scalars.dev]: https://graphql-scalars.dev
[October 2021]: https://spec.graphql.org/October2021
[object safety]: https://doc.rust-lang.org/reference/items/traits.html#object-safety
[orphan rules]: https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules
[Semantic Versioning 2.0.0]: https://semver.org
