enum Character {}

#[juniper::graphql_union]
impl Character {
    fn resolve(&self) {
        match self {}
    }
}

fn main() {}
