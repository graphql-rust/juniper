use juniper::graphql_object;

struct Obj;

#[graphql_object]
impl Obj {
    fn id(&self, __num: i32) -> &str {
        "funA"
    }
}

fn main() {}
