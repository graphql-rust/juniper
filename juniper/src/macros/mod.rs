// Wrapper macros which allows built-in macros to be
// recognized as "crate-local" and helper traits for
// #[juniper::subscription] macro not to recompile them in every impl.

#[macro_use]
mod common;
#[macro_use]
mod object;
#[macro_use]
mod interface;
#[macro_use]
mod scalar;
#[macro_use]
mod union;

#[cfg(feature = "async")]
pub mod subscription_helpers;

#[cfg(test)]
mod tests;
