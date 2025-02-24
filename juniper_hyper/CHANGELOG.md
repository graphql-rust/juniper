`juniper_hyper` changelog
=========================

All user visible changes to `juniper_hyper` crate will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## master

### BC Breaks

- Bumped up [MSRV] to 1.85. ([#1263], [todo])
- Made `hyper::Request` in `graphql()` and `graphql_sync()` functions generic over `B: hyper::body::Body`. ([#1263], [#1102])

[#1102]: /../../issues/1102
[#1263]: /../../pull/1263
[todo]: /../../commit/todo




## [0.9.0] · 2024-03-20
[0.9.0]: /../../tree/juniper_hyper-v0.9.0/juniper_hyper

### BC Breaks

- Switched to 0.16 version of [`juniper` crate].
- Switched to 1 version of [`hyper` crate]. ([#1217])
- Changed return type of all functions from `Response<Body>` to `Response<String>`. ([#1101], [#1096])

[#1096]: /../../issues/1096
[#1101]: /../../pull/1101
[#1217]: /../../pull/1217




## Previous releases

See [old CHANGELOG](/../../blob/juniper_hyper-v0.8.0/juniper_hyper/CHANGELOG.md).




[`juniper` crate]: https://docs.rs/juniper
[`hyper` crate]: https://docs.rs/hyper
[MSRV]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
[Semantic Versioning 2.0.0]: https://semver.org
