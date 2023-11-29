#[derive(juniper::ScalarValue)]
enum ScalarValue {
    Variant(u32, i64),
}

fn main() {}
