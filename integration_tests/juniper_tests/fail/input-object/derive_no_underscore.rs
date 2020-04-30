#[derive(juniper::GraphQLInputObject)]
struct Object {
    #[graphql(name = "__test")]
    test: String,
}

fn main() {}
