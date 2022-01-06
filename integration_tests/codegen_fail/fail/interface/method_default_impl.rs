use juniper::graphql_interface;

#[graphql_interface]
trait Character {
    fn id(&self) -> &str {
        "default"
    }
}

fn main() {}
