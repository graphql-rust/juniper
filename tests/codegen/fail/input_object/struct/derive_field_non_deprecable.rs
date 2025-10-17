use juniper::GraphQLInputObject;

#[derive(GraphQLInputObject)]
struct Object {
    #[graphql(deprecated)]
    test: String,
    #[deprecated]
    other: i32,
}

fn main() {}
