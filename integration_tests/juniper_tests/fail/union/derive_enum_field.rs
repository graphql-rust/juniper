#[derive(juniper::GraphQLEnum)]
pub enum Test {
    A,
    B,
}

#[derive(juniper::GraphQLUnion)]
enum Character {
    Test(Test),
}

fn main() {}
