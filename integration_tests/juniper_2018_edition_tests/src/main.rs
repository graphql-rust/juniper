/// A VERY minimal test case that ensures that
/// macros and custom derives work in 2018 edition crates.
/// This can be removed once juniper is refactored to the 2018 edition.

#[derive(juniper::GraphQLEnum, PartialEq, Eq, Clone, Copy, Debug)]
enum TaskStatus {
    Todo,
    Done,
    Closed,
}

#[derive(juniper::GraphQLObject, Clone, Debug)]
struct Task {
    pub id: i32,
    pub status: TaskStatus,
    pub title: String,
    pub description: Option<String>,
}

#[derive(juniper::GraphQLInputObject, Clone, Debug)]
struct TaskCreate {
    pub title: String,
    pub description: Option<String>,
}

struct Query;

juniper::graphql_object!(Query: () |&self| {
    field task(id: i32) -> Task {
        unimplemented!()
    }
});

fn main() {}
