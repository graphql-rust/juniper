//! Declarative macros and helper definitions for procedural macros.

#[doc(hidden)]
#[macro_use]
pub mod helper;

#[macro_use]
mod graphql_input_value;
#[macro_use]
mod graphql_value;
#[macro_use]
mod graphql_vars;
