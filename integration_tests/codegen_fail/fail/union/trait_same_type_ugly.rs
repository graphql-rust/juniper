use juniper::graphql_union;

#[graphql_union]
trait Character {
    fn a(&self) -> Option<&String>;
    fn b(&self) -> Option<&std::string::String>;
}

fn main() {}
