use juniper::GraphQLInterface;

#[derive(GraphQLInterface)]
#[graphql(for = Node2Value)]
struct Node1 {
    id: String,
}

#[derive(GraphQLInterface)]
#[graphql(impl = Node1Value, for = Node3Value)]
struct Node2 {
    id: String,
}

#[derive(GraphQLInterface)]
#[graphql(impl = Node2Value)]
struct Node3 {
    id: String,
}

fn main() {}
