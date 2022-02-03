#[derive(juniper::GraphQLScalarValue)]
enum ScalarValue {
    Variant { first: i32, second: u64 },
}

fn main() {}
