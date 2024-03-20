`juniper_rocket` changelog
==========================

All user visible changes to `juniper_rocket` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.9.0] · 2024-03-20
[0.9.0]: /../../tree/juniper_rocket-v0.9.0/juniper_rocket

### BC Breaks

- Switched to 0.16 version of [`juniper` crate].
- Switched to 0.5 version of [`rocket` crate]. ([#1205], [#1220])

### Added

- `AsRef` and `AsMut` implementation for `GraphQLRequest` to its inner type. ([#968], [#930])

### Changed

- Made `subscriptions_endpoint_url` argument polymorphic in `graphiql_source()` and `playground_source()`. ([#1223])

[#930]: /../../issues/930
[#968]: /../../pull/968
[#1205]: /../../pull/1205
[#1220]: /../../pull/1220
[#1223]: /../../pull/1223




## Previous releases

See [old CHANGELOG](/../../blob/juniper_rocket-v0.8.2/juniper_rocket/CHANGELOG.md).




[`juniper` crate]: https://docs.rs/juniper
[`rocket` crate]: https://docs.rs/rocket
[Semantic Versioning 2.0.0]: https://semver.org
