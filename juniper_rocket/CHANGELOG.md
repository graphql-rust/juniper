# master

- Compatibility with the latest `juniper`.
- Rocket integration does not require default features.

## Breaking Changes

- `juniper_rocket::graphiql_source` now requires a second parameter for subscriptions

# [[0.5.2] 2019-12-16](https://github.com/graphql-rust/juniper/releases/tag/juniper_rocket-0.5.2)

- Compatibility with the latest `juniper`.

# [[0.5.1] 2019-10-24](https://github.com/graphql-rust/juniper/releases/tag/juniper_rocket-0.5.1)

- Compatibility with the latest `juniper`.

# [[0.5.0] 2019-09-29](https://github.com/graphql-rust/juniper/releases/tag/juniper_rocket-0.5.0)

- Compatibility with the latest `juniper`.

# [[0.4.1] 2019-07-29](https://github.com/graphql-rust/juniper/releases/tag/juniper_rocket-0.4.1)

- Compatibility with the latest `juniper`.

# [[0.4.0] 2019-07-19](https://github.com/graphql-rust/juniper/releases/tag/juniper_rocket-0.4.0)

- Compatibility with the latest `juniper`.

# [[0.3.0] 2019-05-16](https://github.com/graphql-rust/juniper/releases/tag/juniper_rocket-0.3.0)

- Expose the operation names from `GraphQLRequest`.
- Compatibility with the latest `juniper`.

# [0.2.0] 2018-12-17

### Rocket updated to v0.4

[Rocket](https://rocket.rs) integration now requires Rocket `0.4.0`. This is due
to changes with the way Rocket handles form parsing. Before this update, it was
impossible to leverage Rocket integration with Rocket beyond 0.3.x.

Check out [Rocket's Changelog](https://github.com/SergioBenitez/Rocket/blob/v0.4/CHANGELOG.md)
for more details on the 0.4 release.

# juniper_rocket [0.1.3] 2018-09-13

- Add `juniper-0.10.0` compatibility.

# juniper_rocket [0.1.2] 2018-01-13

## Changes

### Rocket updated to `0.3.6`

[Rocket](https://rocket.rs) integration now requires Rocket `0.3.6` to
support building with recent Rust nightlies.

Additional information and supported nightly versions can be found in [Rocket's changelog](https://github.com/SergioBenitez/Rocket/blob/master/CHANGELOG.md#version-036-jan-12-2018).

[#125](https://github.com/graphql-rust/juniper/issues/125)

### Decoding of query params

When processing GET requests, query parameters were not properly url_decoded,

This was fixed by [PR #122](https://github.com/graphql-rust/juniper/pull/128) by @LegNeato.

This fixed the [issue #116](https://github.com/graphql-rust/juniper/issues/116).
