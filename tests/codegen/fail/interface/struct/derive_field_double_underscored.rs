use juniper::GraphQLInterface;

#[derive(GraphQLInterface)]
struct Character {
    __id: String,
}

fn main() {}
