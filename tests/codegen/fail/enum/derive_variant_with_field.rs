use juniper::GraphQLEnum;

#[derive(GraphQLEnum)]
enum Test {
    Variant(i32),
}

fn main() {}
