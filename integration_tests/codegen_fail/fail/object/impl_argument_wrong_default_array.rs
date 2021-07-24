struct Object;

#[juniper::graphql_object]
impl Object {
    #[graphql(arguments(input(default = [true, false, false])))]
    fn wrong(input: [bool; 2]) -> bool {
        input[0]
    }
}

fn main() {}
