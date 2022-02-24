use juniper::GraphQLInterface;

#[derive(GraphQLInterface)]
#[graphql(impl = Node2Value, for = Node2Value)]
struct Node1 {
    id: String,
}

#[derive(GraphQLInterface)]
#[graphql(impl = Node1Value, for = Node1Value)]
struct Node2 {
    id: String,
}

fn main() {}
