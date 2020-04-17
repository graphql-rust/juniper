use crate::{ScalarValue, SchemaType};

pub trait SchemaTranslator<'a, T> {
    fn translate_schema<S: 'a + ScalarValue>(s: &'a SchemaType<S>) -> T;
}

#[cfg(feature = "graphql-parser-integration")]
pub mod graphql_parser;
