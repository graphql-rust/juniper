#[derive(juniper::GraphQLInputObject)]
#[graphql(scalar = juniper::DefaultScalarValue)]
pub struct ObjA {
    test: String,
}

enum Character {
    A(ObjA),
}

juniper::graphql_interface!(Character: () where Scalar = juniper::DefaultScalarValue |&self| {
    field id() -> &str {
        match *self {
            Character::A(_) => "funA",
        }
    }

    instance_resolvers: |_| {
        &ObjA => match *self { Character::A(ref h) => Some(h) },
    }
});

fn main() {}
