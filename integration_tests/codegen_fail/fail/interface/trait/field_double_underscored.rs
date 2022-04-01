use juniper::graphql_interface;

#[graphql_interface]
trait Character {
    fn __id(&self) -> &str;
}

fn main() {}
