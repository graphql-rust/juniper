use juniper::GraphQLEnum;

#[derive(GraphQLEnum)]
enum Test {
    Test,
    #[graphql(name = "TEST")]
    Test1,
}

fn main() {}
