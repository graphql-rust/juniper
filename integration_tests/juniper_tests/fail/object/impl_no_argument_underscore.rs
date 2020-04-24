struct Object {}

#[juniper::graphql_object]
impl Object {
    fn test(&self, __test: String) -> String {
        String::new()
    }
}

fn main() {}
