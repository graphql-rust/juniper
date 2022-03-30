#[derive(juniper::GraphQLEnum)]
enum Test {
    Test,
    #[graphql(name = "TEST")]
    Test1,
}

fn main() {}
