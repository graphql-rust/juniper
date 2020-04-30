struct Object {}

#[juniper::graphql_object]
impl Object {
    #[graphql(arguments(arg(name = "__arg")))]
    fn test(&self, arg: String) -> String {
        arg
    }
}

fn main() {}
