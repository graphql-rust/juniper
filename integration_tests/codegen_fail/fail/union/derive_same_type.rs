#[derive(juniper::GraphQLUnion)]
enum Character {
    A(std::string::String),
    B(String),
}

fn main() {}
