use juniper::GraphQLInputObject;

#[derive(GraphQLInputObject)]
enum Object {
    #[graphql(ignore)]
    Field(String),
}

fn main() {}
