use juniper::GraphQLScalarValue;

#[derive(Clone, Debug, GraphQLScalarValue, PartialEq)]
pub enum DefaultScalarValue {
    Int(i32),
    Float(f64),
    #[graphql(as_str, as_string, into_string)]
    String(String),
    #[graphql(as_boolean)]
    Boolean(bool),
}

fn main() {}
