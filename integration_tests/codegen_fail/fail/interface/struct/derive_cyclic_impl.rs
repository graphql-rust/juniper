use juniper::GraphQLInterface;

#[derive(GraphQLInterface)]
#[graphql(impl = Node1Value, for = [Node2Value, Node3Value])]
struct Node1 {
    id: String,
}

#[derive(GraphQLInterface)]
#[graphql(impl = Node1Value, for = Node3Value)]
struct Node2 {
    id: String,
}

#[derive(GraphQLInterface)]
#[graphql(impl = [Node1Value, Node2Value], for = Node1Value)]
struct Node3 {
    id: String,
}

fn main() {}
