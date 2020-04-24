struct Object {}

#[juniper::graphql_object]
impl Object {
    fn __test(&self) -> String {
        String::new()
    }
}

fn main() {}
