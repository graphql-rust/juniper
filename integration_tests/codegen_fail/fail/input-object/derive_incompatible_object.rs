#[derive(juniper::GraphQLObject)]
struct ObjectA {
    test: String,
}

#[derive(juniper::GraphQLInputObject)]
struct Object {
    field: ObjectA,
}

fn main() {}
