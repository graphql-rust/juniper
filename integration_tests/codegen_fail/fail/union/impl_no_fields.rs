enum Character {}

#[juniper::graphql_union]
impl Character {
    async fn resolve(&self) {
        match self {}
    }
}

fn main() {}
