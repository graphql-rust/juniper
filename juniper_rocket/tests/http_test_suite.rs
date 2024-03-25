use futures::executor;
use juniper::{
    http::tests::{run_http_test_suite, HttpIntegration, TestResponse},
    tests::fixtures::starwars::schema::{Database, Query},
    EmptyMutation, EmptySubscription, RootNode,
};
use juniper_rocket::{GraphQLRequest, GraphQLResponse};
use rocket::{
    get,
    http::ContentType,
    local::asynchronous::{Client, LocalResponse},
    post, routes, Build, Rocket, State,
};

type Schema = RootNode<Query, EmptyMutation<Database>, EmptySubscription<Database>>;

fn bootstrap_rocket() -> Rocket<Build> {
    Rocket::build().manage(Database::new()).manage(Schema::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    ))
}

fn make_rocket() -> Rocket<Build> {
    #[get("/?<request..>")]
    async fn get_handler(
        context: &State<Database>,
        request: GraphQLRequest,
        schema: &State<Schema>,
    ) -> GraphQLResponse {
        request.execute(schema, context).await
    }

    #[post("/", data = "<request>")]
    async fn post_handler(
        context: &State<Database>,
        request: GraphQLRequest,
        schema: &State<Schema>,
    ) -> GraphQLResponse {
        request.execute(schema, context).await
    }

    bootstrap_rocket().mount("/", routes![post_handler, get_handler])
}

fn make_sync_rocket() -> Rocket<Build> {
    #[get("/?<request..>")]
    fn get_handler_sync(
        context: &State<Database>,
        request: GraphQLRequest,
        schema: &State<Schema>,
    ) -> GraphQLResponse {
        request.execute_sync(schema, context)
    }

    #[post("/", data = "<request>")]
    fn post_handler_sync(
        context: &State<Database>,
        request: GraphQLRequest,
        schema: &State<Schema>,
    ) -> GraphQLResponse {
        request.execute_sync(schema, context)
    }

    bootstrap_rocket().mount("/", routes![post_handler_sync, get_handler_sync])
}

struct TestRocketIntegration {
    client: Client,
}

async fn into_test_response(response: LocalResponse<'_>) -> TestResponse {
    let status_code = response.status().code as i32;
    let content_type = response
        .content_type()
        .expect("no `Content-Type` header from handler")
        .to_string();
    let body = response
        .into_string()
        .await
        .expect("no body returned from GraphQL handler");

    TestResponse {
        status_code,
        content_type,
        body: Some(body),
    }
}

impl HttpIntegration for TestRocketIntegration {
    fn get(&self, url: &str) -> TestResponse {
        let req = self.client.get(url);
        let resp = executor::block_on(req.dispatch());
        executor::block_on(into_test_response(resp))
    }

    fn post_json(&self, url: &str, body: &str) -> TestResponse {
        let req = self.client.post(url).header(ContentType::JSON).body(body);
        let resp = executor::block_on(req.dispatch());
        executor::block_on(into_test_response(resp))
    }

    fn post_graphql(&self, url: &str, body: &str) -> TestResponse {
        let req = self
            .client
            .post(url)
            .header(ContentType::new("application", "graphql"))
            .body(body);
        let resp = executor::block_on(req.dispatch());
        executor::block_on(into_test_response(resp))
    }
}

#[rocket::async_test]
async fn test_rocket_integration() {
    let rocket = make_rocket();
    let client = Client::untracked(rocket).await.expect("valid rocket");

    run_http_test_suite(&TestRocketIntegration { client });
}

#[rocket::async_test]
async fn test_sync_rocket_integration() {
    let rocket = make_sync_rocket();
    let client = Client::untracked(rocket).await.expect("valid rocket");

    run_http_test_suite(&TestRocketIntegration { client });
}

#[rocket::async_test]
async fn test_operation_names() {
    #[post("/", data = "<request>")]
    async fn post_graphql_assert_operation_name_handler(
        context: &State<Database>,
        request: GraphQLRequest,
        schema: &State<Schema>,
    ) -> GraphQLResponse {
        assert_eq!(request.operation_names(), vec![Some("TestQuery")]);
        request.execute(schema, context).await
    }

    let rocket = bootstrap_rocket().mount("/", routes![post_graphql_assert_operation_name_handler]);
    let client = Client::untracked(rocket).await.expect("valid rocket");

    let resp = client
        .post("/")
        .header(ContentType::JSON)
        .body(r#"{"query": "query TestQuery {hero{name}}", "operationName": "TestQuery"}"#)
        .dispatch()
        .await;
    let resp = into_test_response(resp).await;

    assert_eq!(resp.status_code, 200, "response: {resp:#?}");
}
