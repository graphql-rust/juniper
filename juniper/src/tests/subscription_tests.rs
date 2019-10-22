
#[derive(GraphQLObject)]
#[graphql(description = "A humanoid creature in the Star Wars universe")]
struct Human {
    id: String,
    name: String,
    home_planet: String,
}

struct MyQuery;

#[juniper::object(
context = MyContext
)]
impl MyQuery {
    fn human(id: String) -> FieldResult<Human> {
        let human = Human {
            id: "query".to_string(),
            name: "Query Human".to_string(),
            home_planet: "Query Human Planet".to_string(),
        };
        Ok(human)
    }
}

struct MyMutation;

#[juniper::object(
context = MyContext
)]
impl MyMutation {
    fn human(id: String) -> FieldResult<Human> {
        let human = Human {
            id: "mutation".to_string(),
            name: "Mutation Human Name".to_string(),
            home_planet: "Mutation Human Planet".to_string(),
        };
        Ok(human)
    }
}

struct MySubscription;

//#[juniper::subscription(
//    context = MyContext
//)]
//impl MySubscription {
//    fn human(id: String) -> Human {
//        let iter = Box::new(std::iter::repeat(Human {
//            id: "subscription id".to_string(),
//            name: "subscription name".to_string(),
//            home_planet: "subscription planet".to_string(),
//        }));
//        Ok(iter)
//    }
//
//    async fn human() -> Human {
//        Ok(Box::pin(futures::stream::repeat(Human {
//            id: "stream human id".to_string(),
//            name: "stream human name".to_string(),
//            home_planet: "stream human home planet".to_string(),
//        })))
//    }
//}

#[test]
fn subscription_returns_iterator() {

}

