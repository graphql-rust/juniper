struct Query;

macro_rules! macro_from_another_project_exporting_items {
    () => {
        pub fn a() {
            1
        }
    
        pub fn b() {
            2
        }
    }
}

#[juniper::graphql_object]
impl Query {
    macro_from_another_project_exporting_items!();
}