use juniper::graphql_interface;

#[graphql_interface]
struct Character {
    __id: String,
}

fn main() {}
