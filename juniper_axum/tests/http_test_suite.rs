use std::sync::Arc;

use axum::{
    body::Body,
    http::Request,
    response::Response,
    routing::{get, post},
    Extension, Router,
};
use futures::TryStreamExt as _;
use juniper::{
    http::tests::{run_http_test_suite, HttpIntegration, TestResponse},
    tests::fixtures::starwars::schema::{Database, Query},
    EmptyMutation, EmptySubscription, RootNode,
};
use juniper_axum::{extract::JuniperRequest, response::JuniperResponse};
use tower_service::Service as _;

type Schema = RootNode<Query, EmptyMutation<Database>, EmptySubscription<Database>>;

struct TestApp(Router);

impl TestApp {
    fn new() -> Self {
        #[axum::debug_handler]
        async fn graphql(
            Extension(schema): Extension<Arc<Schema>>,
            Extension(database): Extension<Database>,
            JuniperRequest(request): JuniperRequest,
        ) -> JuniperResponse {
            JuniperResponse(request.execute(&*schema, &database).await)
        }

        let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());
        let database = Database::new();

        Self(
            Router::new()
                .route("/", get(graphql))
                .route("/", post(graphql))
                .layer(Extension(Arc::new(schema)))
                .layer(Extension(database)),
        )
    }

    fn make_request(&self, req: Request<Body>) -> TestResponse {
        let mut app = self.0.clone();

        let task = app.call(req);

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                // PANIC: Unwrapping is OK here, because `task` is `Infallible`.
                let resp = task.await.unwrap();
                into_test_response(resp).await
            })
    }
}

impl HttpIntegration for TestApp {
    fn get(&self, url: &str) -> TestResponse {
        let req = Request::get(url).body(Body::empty()).unwrap();
        self.make_request(req)
    }

    fn post_json(&self, url: &str, body: &str) -> TestResponse {
        let req = Request::post(url)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        self.make_request(req)
    }

    fn post_graphql(&self, url: &str, body: &str) -> TestResponse {
        let req = Request::post(url)
            .header("content-type", "application/graphql")
            .body(Body::from(body.to_string()))
            .unwrap();
        self.make_request(req)
    }
}

/// Converts the provided [`Response`] into to a [`TestResponse`].
async fn into_test_response(resp: Response) -> TestResponse {
    let status_code = resp.status().as_u16().into();

    let content_type: String = resp
        .headers()
        .get("content-type")
        .map(|header| {
            header
                .to_str()
                .unwrap_or_else(|e| panic!("not UTF-8 header: {e}"))
                .to_owned()
        })
        .unwrap_or_default();

    let body = String::from_utf8(
        resp.into_body()
            .into_data_stream()
            .map_ok(|bytes| bytes.to_vec())
            .try_concat()
            .await
            .unwrap(),
    )
    .unwrap_or_else(|e| panic!("not UTF-8 body: {e}"));

    TestResponse {
        status_code,
        content_type,
        body: Some(body),
    }
}

#[test]
fn test_axum_integration() {
    run_http_test_suite(&TestApp::new())
}
