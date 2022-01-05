use juniper::graphql_interface;

#[graphql_interface]
trait Character {
    fn wrong(&self, #[graphql(default = [true, false, false])] input: [bool; 2]) -> bool {
        input[0]
    }
}

fn main() {}
