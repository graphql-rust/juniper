//! Provides GraphQLType implementations for some external types
#[doc(hidden)]
pub mod serde;

#[cfg(feature = "bigdecimal")]
/// GraphQL support for [bigdecimal](https://github.com/akubera/bigdecimal-rs) types.
pub mod bigdecimal;

#[cfg(feature = "chrono")]
/// GraphQL support for [chrono](https://github.com/chronotope/chrono) types.
pub mod chrono;

#[cfg(feature = "url")]
/// GraphQL support for [url](https://github.com/servo/rust-url) types.
pub mod url;

#[cfg(feature = "uuid")]
/// GraphQL support for [uuid](https://doc.rust-lang.org/uuid/uuid/struct.Uuid.html) types.
pub mod uuid;
