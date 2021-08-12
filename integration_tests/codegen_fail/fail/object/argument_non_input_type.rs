use juniper::{graphql_object, GraphQLObject};

#[derive(GraphQLObject)]
struct ObjA {
    test: String,
}

struct ObjB;

#[graphql_object]
impl ObjB {
    fn id(&self, obj: ObjA) -> &str {
        "funA"
    }
}

fn main() {}
