use juniper::graphql_interface;

#[graphql_interface(for = Node2Value)]
struct Node1 {
    id: String,
}

#[graphql_interface(impl = Node1Value, for = Node3Value)]
struct Node2 {
    id: String,
}

#[graphql_interface(impl = Node2Value)]
struct Node3 {
    id: String,
}

fn main() {}
