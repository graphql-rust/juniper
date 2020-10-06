use juniper::http::{GraphQLRequest, GraphQLResponse};
use juniper::Document;
use juniper::{
    graphql_object, DefaultScalarValue, EmptyMutation, EmptySubscription, ParseError, RootNode,
    Spanning,
};

#[derive(Clone, Copy, Debug)]
struct Context;
impl juniper::Context for Context {}

#[derive(Clone, Debug)]
struct User {
    name: String,
}

#[graphql_object(Context = Context)]
impl User {
    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Copy, Debug)]
struct Query;

#[graphql_object(Context = Context)]
impl Query {
    fn users() -> Vec<User> {
        vec![User {
            name: "user1".into(),
        }]
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;

fn main() {
    let schema = Schema::new(
        Query,
        EmptyMutation::<Context>::new(),
        EmptySubscription::<Context>::new(),
    );
    let ctx = Context {};
    let query = r#" query { users { name } } "#;
    let req = GraphQLRequest::<DefaultScalarValue>::new(query.to_owned(), None, None);

    {
        // Just parse the request query
        let doc: Result<Document<DefaultScalarValue>, Spanning<ParseError>> = req.parse(&schema);
        println!("{:#?}", &doc);
    }

    {
        // Execute the query synchronously
        let res: GraphQLResponse = req.execute_sync(&schema, &ctx);
        println!("{:#?}", &res);
    }
}
