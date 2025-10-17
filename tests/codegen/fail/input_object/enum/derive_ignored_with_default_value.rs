use juniper::GraphQLInputObject;

#[derive(GraphQLInputObject)]
enum Object {
    Num(i32),
    #[graphql(ignore, default = "none")]
    Test(String),
    #[graphql(skip, default)]
    Test2(String),
}

fn main() {}
