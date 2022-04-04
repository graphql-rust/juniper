use juniper::graphql_interface;

#[graphql_interface]
trait Character {
    fn id(&self) -> &str;

    #[graphql(name = "id")]
    fn id2(&self) -> &str;
}

fn main() {}
