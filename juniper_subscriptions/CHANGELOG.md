`juniper_subscriptions` changelog
=================================

All user visible changes to `juniper_subscriptions` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.18.0] · 2025-09-08
[0.18.0]: /../../tree/juniper_subscriptions-v0.18.0/juniper_subscriptions

### BC Breaks

- Switched to 0.17 version of [`juniper` crate].
- Removed lifetime parameters from `Coordinator`. ([#1247], [#819])
- Bumped up [MSRV] to 1.85. ([#1272], [1b1fc618])

[#819]: /../../issues/819
[#1247]: /../../pull/1247
[#1272]: /../../pull/1272
[1b1fc618]: /../../commit/1b1fc61879ffdd640d741e187dc20678bf7ab295




## [0.17.0] · 2024-03-20
[0.17.0]: /../../tree/juniper_subscriptions-v0.17.0/juniper_subscriptions

### BC Breaks

- Switched to 0.16 version of [`juniper` crate].




## Previous releases

See [old CHANGELOG](/../../blob/juniper_subscriptions-v0.16.0/juniper_subscriptions/CHANGELOG.md).




[`juniper` crate]: https://docs.rs/juniper
[MSRV]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
[Semantic Versioning 2.0.0]: https://semver.org
