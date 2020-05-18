struct Object {}

#[juniper::graphql_object]
impl Object {
    #[graphql(name = "__test")]
    async fn test(&self) -> String {
        String::new()
    }
}

fn main() {}
