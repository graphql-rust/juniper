use juniper::GraphQLUnion;

#[derive(GraphQLUnion)]
enum Character {
    A(u8),
    B(u8),
}

fn main() {}
