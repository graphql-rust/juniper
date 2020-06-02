use juniper::GraphQLUnion;

#[derive(GraphQLUnion)]
enum Character {
    A(std::string::String),
    B(String),
}

fn main() {}
