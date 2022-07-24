use axum::{
    http::Request,
    response::Response,
    routing::{get, post},
    Extension, Router,
};
use hyper::{service::Service, Body};
use juniper::{
    http::tests::{run_http_test_suite, HttpIntegration, TestResponse},
    tests::fixtures::starwars::schema::{Database, Query},
    EmptyMutation, EmptySubscription, RootNode,
};
use juniper_axum::{extract::JuniperRequest, response::JuniperResponse};
use std::{str::from_utf8, sync::Arc};

/// The app we want to test
struct AxumApp(Router);

type Schema = RootNode<'static, Query, EmptyMutation<Database>, EmptySubscription<Database>>;

/// Create a new axum app to test
fn test_app() -> AxumApp {
    let schema = Schema::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    let context = Database::new();

    let router = Router::new()
        .route("/", get(graphql))
        .route("/", post(graphql))
        .layer(Extension(Arc::from(schema)))
        .layer(Extension(Arc::from(context)));

    AxumApp(router)
}

async fn graphql(
    JuniperRequest(request): JuniperRequest,
    Extension(schema): Extension<Arc<Schema>>,
    Extension(context): Extension<Arc<Database>>,
) -> JuniperResponse {
    JuniperResponse(request.execute(&schema, &context).await)
}

/// Implement HttpIntegration to enable standard tests
impl HttpIntegration for AxumApp {
    fn get(&self, url: &str) -> TestResponse {
        let request = Request::get(url).body(Body::empty()).unwrap();

        self.make_request(request)
    }

    fn post_json(&self, url: &str, body: &str) -> TestResponse {
        let request = Request::post(url)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        self.make_request(request)
    }

    fn post_graphql(&self, url: &str, body: &str) -> TestResponse {
        let request = Request::post(url)
            .header("content-type", "application/graphql")
            .body(Body::from(body.to_string()))
            .unwrap();

        self.make_request(request)
    }
}

impl AxumApp {
    /// Make a request to the Axum app
    fn make_request(&self, request: Request<Body>) -> TestResponse {
        let mut app = self.0.clone();

        let task = app.call(request);

        // Call async code with tokio runtime
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let response = task.await.unwrap();
                create_test_response(response).await
            })
    }
}

/// Convert an Axum Response to a Juniper TestResponse
async fn create_test_response(response: Response) -> TestResponse {
    let status_code: i32 = response.status().as_u16().into();
    let content_type: String = response
        .headers()
        .get("content-type")
        .map(|header| from_utf8(header.as_bytes()).unwrap().to_string())
        .unwrap_or_default();

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body: Option<String> = Some(from_utf8(&body).map(|s| s.to_string()).unwrap());

    TestResponse {
        status_code,
        content_type,
        body,
    }
}

#[test]
fn test_axum_integration() {
    let test_app = test_app();
    run_http_test_suite(&test_app)
}
