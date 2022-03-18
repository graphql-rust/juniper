#[derive(juniper::ScalarValue)]
enum ScalarValue {
    Variant { first: i32, second: u64 },
}

fn main() {}
