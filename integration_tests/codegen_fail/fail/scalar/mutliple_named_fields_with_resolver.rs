use juniper::{GraphQLScalar, Value};

#[derive(GraphQLScalar)]
#[graphql(to_output_with = Self::to_output)]
struct Scalar {
    id: i32,
    another: i32,
}

impl Scalar {
    fn to_output(&self) -> Value {
        Value::scalar(self.id)
    }
}

fn main() {}
