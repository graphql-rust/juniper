use juniper::graphql_interface;

#[graphql_interface(for = Node2Value)]
trait Node1 {
    fn id() -> String;
}

#[graphql_interface(impl = Node1Value, for = Node3Value)]
trait Node2 {
    fn id(&self) -> &str;
}

#[graphql_interface(impl = Node2Value)]
trait Node3 {
    fn id() -> &'static str;
}

fn main() {}
