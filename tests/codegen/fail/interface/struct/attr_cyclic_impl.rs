use juniper::graphql_interface;

#[graphql_interface(impl = Node2Value, for = Node2Value)]
struct Node1 {
    id: String,
}

#[graphql_interface(impl = Node1Value, for = Node1Value)]
struct Node2 {
    id: String,
}

fn main() {}
