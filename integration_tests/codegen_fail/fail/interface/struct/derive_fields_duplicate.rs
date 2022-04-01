use juniper::GraphQLInterface;

#[derive(GraphQLInterface)]
struct Character {
    id: String,

    #[graphql(name = "id")]
    id2: String,
}

fn main() {}
