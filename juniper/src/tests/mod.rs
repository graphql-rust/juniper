//! Library tests and fixtures

// todo#433: uncomment tests after rebase and check if they compile and pass
//#[cfg(test)]
//mod introspection_tests;
pub mod model;
//#[cfg(test)]
//mod query_tests;
//pub mod schema;
#[cfg(test)]
mod schema_introspection;
#[cfg(test)]
mod type_info_tests;

#[cfg(all(test, feature = "async"))]
mod subscription;
