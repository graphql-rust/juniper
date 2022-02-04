use juniper::{GraphQLScalar, Value};

#[derive(GraphQLScalar)]
#[graphql(to_output_with = Self::to_output)]
struct Scalar(i32, i32);

impl Scalar {
    fn to_output(&self) -> Value {
        Value::scalar(self.0)
    }
}

fn main() {}
