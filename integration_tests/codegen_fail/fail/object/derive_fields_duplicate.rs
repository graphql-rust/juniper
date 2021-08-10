use juniper::GraphQLObject;

#[derive(GraphQLObject)]
struct ObjA {
    id: String,
    #[graphql(name = "id")]
    id2: String,
}

fn main() {}
