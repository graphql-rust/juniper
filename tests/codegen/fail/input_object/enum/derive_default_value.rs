use juniper::GraphQLInputObject;

#[derive(GraphQLInputObject)]
enum Object {
    #[graphql(default)]
    Test(String),
}

fn main() {}
