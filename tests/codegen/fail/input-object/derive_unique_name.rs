use juniper::GraphQLInputObject;

#[derive(GraphQLInputObject)]
struct Object {
    test: String,
    #[graphql(name = "test")]
    test2: String,
}

fn main() {}
