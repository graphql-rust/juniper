use juniper::GraphQLInputObject;

#[derive(GraphQLInputObject)]
union Test { id: i32 }

fn main() {}
