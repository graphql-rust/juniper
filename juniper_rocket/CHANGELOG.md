# [master]

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
