use crate::{ScalarValue, SchemaType};

#[cfg_attr(not(feature = "schema-language"), allow(dead_code))]
pub trait SchemaTranslator<'a, T> {
    fn translate_schema<S: 'a + ScalarValue>(s: &'a SchemaType<S>) -> T;
}

#[cfg(feature = "schema-language")]
pub mod graphql_parser;
