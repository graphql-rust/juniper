struct Object;

#[juniper::graphql_object]
impl Object {
    fn wrong(#[graphql(default = [true, false, false])] input: [bool; 2]) -> bool {
        input[0]
    }
}

fn main() {}
