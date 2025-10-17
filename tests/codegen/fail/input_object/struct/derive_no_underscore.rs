use juniper::GraphQLInputObject;

#[derive(GraphQLInputObject)]
struct Object {
    #[graphql(name = "__test")]
    test: String,
}

fn main() {}
