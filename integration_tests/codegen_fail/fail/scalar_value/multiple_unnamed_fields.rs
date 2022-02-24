#[derive(juniper::GraphQLScalarValue)]
enum ScalarValue {
    Variant(u32, i64),
}

fn main() {}
