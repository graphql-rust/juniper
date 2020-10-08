use crate::SchemaType;

pub trait SchemaTranslator<'a, T> {
    fn translate_schema(s: &'a SchemaType) -> T;
}

#[cfg(feature = "graphql-parser-integration")]
pub mod graphql_parser;
