use juniper::graphql_interface;

#[graphql_interface]
trait Character {
    fn id(&self, #[graphql(deprecated)] num: i32) -> &str;
    async fn name(&self, #[graphql(deprecated = "reason")] pre: Vec<String>) -> &str;
}

fn main() {}
