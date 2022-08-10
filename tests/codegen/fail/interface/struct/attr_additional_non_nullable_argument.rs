use juniper::{graphql_interface, graphql_object};

pub struct ObjA {
    id: String,
}

#[graphql_object(impl = CharacterValue)]
impl ObjA {
    fn id(&self, is_present: bool) -> &str {
        is_present.then_some(&*self.id).unwrap_or("missing")
    }
}

#[graphql_interface(for = ObjA)]
struct Character {
    id: String,
}

fn main() {}
