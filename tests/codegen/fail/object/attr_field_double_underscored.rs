use juniper::graphql_object;

struct ObjA;

#[graphql_object]
impl Character for ObjA {
    fn __id(&self) -> &str {
        "funA"
    }
}

fn main() {}
