//! Provides GraphQLType implementations for some external types
#[doc(hidden)]
pub mod serde;

#[cfg(feature = "chrono")]
/// GraphQL support for [chrono](https://github.com/chronotope/chrono) types.
pub mod chrono;

#[cfg(feature = "url")]
/// GraphQL support for [url](https://github.com/servo/rust-url) types.
pub mod url;

#[cfg(feature = "uuid")]
/// GraphQL support for [uuid](https://doc.rust-lang.org/uuid/uuid/struct.Uuid.html) types.
pub mod uuid;

#[cfg(feature = "humantime")]
/// GraphQL support for stdlib timestamps via [humantime](https://crates.io/crates/humantime) types.
pub mod humantime;
