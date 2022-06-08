//! Declarative macros and helper definitions for procedural macros.

#[doc(hidden)]
pub mod helper;
#[doc(hidden)]
#[macro_use]
pub mod reflect;

mod input_value;
mod value;
mod vars;

#[doc(inline)]
pub use self::{input_value::input_value, value::value, vars::vars};
