use juniper::graphql_union;

#[graphql_union]
trait Character {
    fn a(&self) -> Option<&u8>;
    fn b(&self) -> Option<&u8>;
}

fn main() {}
