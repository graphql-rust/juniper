use juniper::graphql_interface;

#[graphql_interface]
struct __Character {
    id: String,
}

fn main() {}
