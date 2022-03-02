use juniper::ScalarValue;

#[derive(Clone, Debug, ScalarValue, PartialEq)]
pub enum DefaultScalarValue {
    Int(i32),
    Float(f64),
    #[value(as_str, as_string, into_string)]
    String(String),
    #[value(as_bool)]
    Boolean(bool),
}

fn main() {}
