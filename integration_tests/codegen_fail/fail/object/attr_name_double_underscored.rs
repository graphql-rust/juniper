use juniper::graphql_object;

struct __Obj;

#[graphql_object]
impl __Obj {
    fn id(&self) -> &str {
        "funA"
    }
}

fn main() {}
