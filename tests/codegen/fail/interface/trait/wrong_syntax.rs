use juniper::graphql_interface;

#[graphql_interface]
trait Character {
    fn id(&self) -> &str;

    #[graphql(ignore)]
    fn id2(&self) -> &str {
        self.self.id()
    }
}

fn main() {}
