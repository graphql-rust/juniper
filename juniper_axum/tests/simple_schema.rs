use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
    Extension, Router,
};
use juniper::{graphql_object, EmptyMutation, EmptySubscription, RootNode};
use juniper_axum::{extract::JuniperRequest, playground, response::JuniperResponse};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::util::ServiceExt;

const GRAPHQL_ENDPOINT: &str = "/graphql";

pub struct Context();

impl juniper::Context for Context {}
pub struct Query;

#[graphql_object(context = Context)]
impl Query {
    fn add(a: i32, b: i32) -> i32 {
        a + b
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;

fn app() -> Router {
    let schema = Arc::from(Schema::new(
        Query,
        EmptyMutation::<Context>::new(),
        EmptySubscription::<Context>::new(),
    ));

    let context = Arc::new(Context());

    Router::new()
        .route("/", get(|| playground(GRAPHQL_ENDPOINT, None)))
        .route(GRAPHQL_ENDPOINT, post(graphql))
        .layer(Extension(schema))
        .layer(Extension(context))
}

async fn graphql(
    JuniperRequest(request): JuniperRequest,
    Extension(schema): Extension<Arc<Schema>>,
    Extension(context): Extension<Arc<Context>>,
) -> JuniperResponse {
    JuniperResponse(request.execute(&schema, &context).await)
}

#[tokio::test]
async fn add_two_and_three() {
    let app = app();

    let request_json = Body::from(r#"{ "query": "{ add(a: 2, b: 3) }"}"#);
    let request = Request::post(GRAPHQL_ENDPOINT)
        .header("content-type", "application/json")
        .body(request_json)
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body, json!({ "data": { "add": 5 } }));
}

#[tokio::test]
async fn playground_is_ok() {
    let app = app();

    let request = Request::get("/").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
