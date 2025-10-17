use juniper::GraphQLInputObject;

#[derive(GraphQLInputObject)]
enum Object {
    #[graphql(name = "__test")]
    Test(String),
}

fn main() {}
