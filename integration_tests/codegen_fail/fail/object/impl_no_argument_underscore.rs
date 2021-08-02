struct Object {}

#[juniper::graphql_object]
impl Object {
    fn test(&self, #[graphql(name = "__arg")] arg: String) -> String {
        arg
    }
}

fn main() {}
