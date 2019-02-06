use schema::model::SchemaType;
use value::ScalarValue;

pub trait SchemaTranslator<T> {
    fn translate_schema<'a, S: ScalarValue>(s: &SchemaType<'a, S>) -> T;
}

#[cfg(feature = "graphql-parser-integration")]
pub mod graphql_parser;
