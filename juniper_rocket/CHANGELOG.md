`juniper_rocket` changelog
==========================

All user visible changes to `juniper_rocket` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## master

### BC Breaks

- Switched to 0.16 version of [`juniper` crate].
- Switched to 0.5.0-rc.4 version of [`rocket` crate]. ([#1205])

### Added

- `AsRef` and `AsMut` implementation for `GraphQLRequest` to its inner type. ([#968], [#930])

[#930]: /../../issues/930
[#968]: /../../pull/968
[#1205]: /../../pull/1205




## Previous releases

See [old CHANGELOG](/../../blob/juniper_rocket-v0.8.2/juniper_rocket/CHANGELOG.md).




[`juniper` crate]: https://docs.rs/juniper
[`rocket` crate]: https://docs.rs/rocket
[Semantic Versioning 2.0.0]: https://semver.org
