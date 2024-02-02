use juniper::graphql_object;

struct ObjA;

#[graphql_object]
impl ObjA {
    fn __id(&self) -> &str {
        "funA"
    }
}

fn main() {}
