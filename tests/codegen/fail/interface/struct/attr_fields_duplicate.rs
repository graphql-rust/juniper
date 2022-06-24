use juniper::graphql_interface;

#[graphql_interface]
struct Character {
    id: String,

    #[graphql(name = "id")]
    id2: String,
}

fn main() {}
