use juniper::{graphql_object, GraphQLInputObject};

#[derive(GraphQLInputObject)]
struct ObjB {
    id: i32,
}

struct ObjA;

#[graphql_object]
impl ObjA {
    fn id(&self) -> ObjB {
        ObjB { id: 34 }
    }
}

fn main() {}
