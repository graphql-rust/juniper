use juniper::ScalarValue;

#[derive(Clone, Debug, ScalarValue, PartialEq)]
pub enum DefaultScalarValue {
    Int(i32),
    Float(f64),
    #[scalar_value(as_str, as_string, into_string)]
    String(String),
    #[scalar_value(as_bool)]
    Boolean(bool),
}

fn main() {}
