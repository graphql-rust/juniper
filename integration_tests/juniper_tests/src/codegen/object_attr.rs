use juniper::graphql_object;

mod two_objects_on_generic {
    use super::*;

    struct Generic<T>(T);

    #[graphql_object(name = "BooleanGeneric")]
    impl Generic<bool> {
        fn boolean(&self) -> bool {
            self.0
        }
    }

    #[graphql_object(name = "IntGeneric")]
    impl Generic<i32> {
        fn num(&self) -> i32 {
            self.0
        }
    }
}
