// Wrapper macros which allows built-in macros to be recognized as "crate-local"
// and helper traits for #[juniper::graphql_subscription] macro.

#[macro_use]
mod common;
#[macro_use]
mod interface;
#[macro_use]
mod scalar;

#[cfg(test)]
mod tests;

pub mod subscription_helpers;
