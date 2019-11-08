// Wrapper macros which allows built-in macros to be recognized as "crate-local".

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

#[cfg(test)]
mod tests;
