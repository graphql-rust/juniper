struct Object {}

#[juniper::graphql_object]
impl Object {
    async fn test(&self) -> String {
        String::new()
    }

    async fn test(&self) -> String {
        String::new()
    }
}

fn main() {}
