use juniper::graphql_object;

struct ObjA;

#[graphql_object]
impl ObjA {
    fn id(&self) -> &str {
        "funA"
    }

    #[graphql(name = "id")]
    fn id2(&self) -> &str {
        "funB"
    }
}

fn main() {}
