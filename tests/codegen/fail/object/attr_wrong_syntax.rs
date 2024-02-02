use juniper::graphql_object;

struct MyObject {
    my_field: i32,
}

#[graphql_object]
impl MyObject {
    fn my_field(&self) -> i32 {
        self.self.my_field
    }
}

fn main() {}
