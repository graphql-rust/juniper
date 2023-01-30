`juniper_warp` changelog
========================

All user visible changes to `juniper_warp` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## master

### BC Breaks

- Switched to 0.16 version of [`juniper` crate].

### Changed

- Made `schema` argument of `make_graphql_filter()` and `make_graphql_filter_sync()` polymorphic, allowing to specify external `Arc`ed `schema`. ([#1136], [#1135])

[#1135]: /../../issues/1136
[#1136]: /../../pull/1136




## Previous releases

See [old CHANGELOG](/../../blob/juniper_warp-v0.7.0/juniper_warp/CHANGELOG.md).




[`juniper` crate]: https://docs.rs/juniper
[Semantic Versioning 2.0.0]: https://semver.org
