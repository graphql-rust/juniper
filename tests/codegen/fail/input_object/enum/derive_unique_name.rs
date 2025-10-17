use juniper::GraphQLInputObject;

#[derive(GraphQLInputObject)]
enum Object {
    Test(String),
    #[graphql(name = "test")]
    Test2(String),
}

fn main() {}
