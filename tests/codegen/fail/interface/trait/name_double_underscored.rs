use juniper::graphql_interface;

#[graphql_interface]
trait __Character {
    fn id(&self) -> &str;
}

fn main() {}
