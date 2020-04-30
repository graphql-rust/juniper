#[derive(juniper::GraphQLObject)]
struct Object {
    #[graphql(name = "__test")]
    test: String,
}

fn main() {}
