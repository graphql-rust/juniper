use juniper::{GraphQLScalar, Value};

#[derive(GraphQLScalar)]
#[graphql(resolve = Self::resolve)]
struct Scalar {
    id: i32,
    another: i32,
}

impl Scalar {
    fn resolve(&self) -> Value {
        Value::scalar(self.id)
    }
}

fn main() {}
