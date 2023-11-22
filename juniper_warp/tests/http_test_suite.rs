use juniper::{
    http::tests::{run_http_test_suite, HttpIntegration, TestResponse},
    tests::fixtures::starwars::schema::{Database, Query},
    EmptyMutation, EmptySubscription, RootNode,
};
use juniper_warp::{make_graphql_filter, make_graphql_filter_sync};
use warp::{
    body,
    filters::{path, BoxedFilter},
    http, reply, Filter,
};

struct TestWarpIntegration {
    filter: BoxedFilter<(reply::Response,)>,
}

impl TestWarpIntegration {
    fn new(is_sync: bool) -> Self {
        let schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );
        let db = warp::any().map(Database::new);

        Self {
            filter: path::end()
                .and(if is_sync {
                    make_graphql_filter_sync(schema, db).boxed()
                } else {
                    make_graphql_filter(schema, db).boxed()
                })
                .boxed(),
        }
    }

    fn make_request(&self, req: warp::test::RequestBuilder) -> TestResponse {
        let rt = tokio::runtime::Runtime::new()
            .unwrap_or_else(|e| panic!("failed to create `tokio::Runtime`"));
        make_test_response(rt.block_on(async move {
            req.filter(&self.filter).await.unwrap_or_else(|rejection| {
                let code = if rejection.is_not_found() {
                    http::StatusCode::NOT_FOUND
                } else if let Some(body::BodyDeserializeError { .. }) = rejection.find() {
                    http::StatusCode::BAD_REQUEST
                } else {
                    http::StatusCode::INTERNAL_SERVER_ERROR
                };
                http::Response::builder()
                    .status(code)
                    .header("content-type", "application/json")
                    .body(Vec::new().into())
                    .unwrap()
            })
        }))
    }
}

impl HttpIntegration for TestWarpIntegration {
    fn get(&self, url: &str) -> TestResponse {
        use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
        use url::Url;

        /// https://url.spec.whatwg.org/#query-state
        const QUERY_ENCODE_SET: &AsciiSet =
            &CONTROLS.add(b' ').add(b'"').add(b'#').add(b'<').add(b'>');

        let url = Url::parse(&format!("http://localhost:3000{url}")).expect("url to parse");

        let url: String = utf8_percent_encode(url.query().unwrap_or(""), QUERY_ENCODE_SET)
            .collect::<Vec<_>>()
            .join("");

        self.make_request(
            warp::test::request()
                .method("GET")
                .path(&format!("/?{url}")),
        )
    }

    fn post_json(&self, url: &str, body: &str) -> TestResponse {
        self.make_request(
            warp::test::request()
                .method("POST")
                .header("content-type", "application/json; charset=utf-8")
                .path(url)
                .body(body),
        )
    }

    fn post_graphql(&self, url: &str, body: &str) -> TestResponse {
        self.make_request(
            warp::test::request()
                .method("POST")
                .header("content-type", "application/graphql; charset=utf-8")
                .path(url)
                .body(body),
        )
    }
}

fn make_test_response(resp: reply::Response) -> TestResponse {
    TestResponse {
        status_code: resp.status().as_u16() as i32,
        body: Some(String::from_utf8(resp.body().to_owned()).unwrap()),
        content_type: resp
            .headers()
            .get("content-type")
            .expect("missing content-type header in warp response")
            .to_str()
            .expect("invalid content-type string")
            .into(),
    }
}

#[test]
fn test_warp_integration() {
    run_http_test_suite(&TestWarpIntegration::new(false));
}

#[test]
fn test_sync_warp_integration() {
    run_http_test_suite(&TestWarpIntegration::new(true));
}
