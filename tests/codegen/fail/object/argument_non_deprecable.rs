use juniper::graphql_object;

struct Character;

#[graphql_object]
impl Character {
    fn id(&self, #[graphql(deprecated)] num: i32) -> &str {
        _ = num;
        "123"
    }

    async fn name(&self, #[graphql(deprecated = "reason")] pre: Vec<String>) -> &str {
        _ = pre;
        "whatever"
    }
}

fn main() {}
