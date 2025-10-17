use juniper::GraphQLInputObject;

#[derive(GraphQLInputObject)]
struct Object {
    #[graphql(ignore)]
    field: String,
}

fn main() {}
