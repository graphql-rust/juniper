use juniper::graphql_interface;

#[graphql_interface(impl = Node2Value, for = Node2Value)]
trait Node1 {
    fn id(&self) -> &str;
}

#[graphql_interface(impl = Node1Value, for = Node1Value)]
trait Node2 {
    fn id() -> String;
}

fn main() {}
