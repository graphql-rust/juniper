#[macro_use]
extern crate juniper;
#[cfg(test)]
#[macro_use]
extern crate serde_json;

#[cfg(test)]
extern crate fnv;
#[cfg(test)]
extern crate indexmap;

mod codegen;
mod custom_scalar;
