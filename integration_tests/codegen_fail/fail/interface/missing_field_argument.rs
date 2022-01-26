use juniper::{graphql_interface, graphql_object};

pub struct ObjA {
    id: String,
}

#[graphql_object(impl = CharacterValue)]
impl ObjA {
    fn id(&self) -> &String {
        &self.id
    }
}

#[graphql_interface(for = ObjA)]
trait Character {
    fn id(&self, is_present: bool) -> &str;
}

fn main() {}
