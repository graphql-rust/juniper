use juniper::graphql_object;

struct ObjA;

#[graphql_object]
impl ObjA {
    fn wrong(&self, #[graphql(default = [true, false, false])] input: [bool; 2]) -> bool {
        input[0]
    }
}

fn main() {}
