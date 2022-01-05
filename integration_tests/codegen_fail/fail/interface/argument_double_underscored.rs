use juniper::graphql_interface;

#[graphql_interface]
trait Character {
    fn id(&self, __num: i32) -> &str {
        "funA"
    }
}

fn main() {}
