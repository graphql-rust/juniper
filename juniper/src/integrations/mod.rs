//! Provides GraphQLType implementations for some external types

#[cfg(feature = "bson")]
pub mod bson;
#[cfg(feature = "chrono")]
pub mod chrono;
#[cfg(feature = "chrono-tz")]
pub mod chrono_tz;
#[cfg(feature = "json")]
pub mod json;
#[doc(hidden)]
pub mod serde;
#[cfg(feature = "time")]
pub mod time;
#[cfg(feature = "url")]
pub mod url;
#[cfg(feature = "uuid")]
pub mod uuid;
