use derive_more::{Display, From, TryInto};
use juniper::ScalarValue;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Display, From, PartialEq, ScalarValue, Serialize, TryInto)]
pub enum DefaultScalarValue {
    Int(i32),
    #[value(to_float)]
    Float(f64),
    #[value(as_str, to_string)]
    String(String),
    #[value(to_bool)]
    Boolean(bool),
}

fn main() {}
