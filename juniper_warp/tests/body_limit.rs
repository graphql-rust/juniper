//! Tests of the request body limit of the GraphQL filters.

#![allow(unused_crate_dependencies, reason = "integration tests")]

use juniper::{
    EmptyMutation, EmptySubscription, RootNode,
    tests::fixtures::starwars::schema::{Database, Query},
};
use juniper_warp::{
    DEFAULT_BODY_LIMIT, make_graphql_filter, make_graphql_filter_sync,
    make_graphql_filter_sync_with_body_limit, make_graphql_filter_with_body_limit,
};
use warp::{
    Filter as _,
    filters::BoxedFilter,
    http::{Response, StatusCode},
    hyper::body::Bytes,
    reply,
    test::request,
};

type Schema = RootNode<Query, EmptyMutation<Database>, EmptySubscription<Database>>;

type GraphQLFilter = BoxedFilter<(reply::Response,)>;

const CONTENT_TYPES: [&str; 2] = ["application/json", "application/graphql"];

/// Custom body limit of the tests.
const LIMIT: usize = 64;

fn schema() -> Schema {
    Schema::new(Query, EmptyMutation::new(), EmptySubscription::new())
}

/// Returns the asynchronous and the synchronous filters with the default body limit.
fn default_filters() -> [GraphQLFilter; 2] {
    [
        make_graphql_filter(schema(), warp::any().map(Database::new)).boxed(),
        make_graphql_filter_sync(schema(), warp::any().map(Database::new)).boxed(),
    ]
}

/// Returns the asynchronous and the synchronous filters with the `limit`.
fn filters(limit: usize) -> [GraphQLFilter; 2] {
    [
        make_graphql_filter_with_body_limit(schema(), warp::any().map(Database::new), limit)
            .boxed(),
        make_graphql_filter_sync_with_body_limit(schema(), warp::any().map(Database::new), limit)
            .boxed(),
    ]
}

/// Returns a valid body of `size` bytes for the `content_type`.
///
/// Trailing spaces pad the query to `size` bytes. JSON and GraphQL ignore the spaces.
fn padded_body(content_type: &str, size: usize) -> String {
    let query = match content_type {
        "application/json" => r#"{"query": "{ hero { name } }"}"#,
        _ => "{ hero { name } }",
    };
    format!("{query}{}", " ".repeat(size - query.len()))
}

async fn post(filter: &GraphQLFilter, content_type: &str, size: usize) -> Response<Bytes> {
    request()
        .method("POST")
        .header("content-type", content_type)
        .body(padded_body(content_type, size))
        .reply(filter)
        .await
}

#[tokio::test]
async fn accepts_body_at_default_limit() {
    for filter in default_filters() {
        for content_type in CONTENT_TYPES {
            let resp = post(&filter, content_type, DEFAULT_BODY_LIMIT).await;

            assert_eq!(resp.status(), StatusCode::OK, "{content_type}");
        }
    }
}

#[tokio::test]
async fn rejects_body_over_default_limit() {
    for filter in default_filters() {
        for content_type in CONTENT_TYPES {
            let resp = post(&filter, content_type, DEFAULT_BODY_LIMIT + 1).await;

            assert_eq!(
                resp.status(),
                StatusCode::PAYLOAD_TOO_LARGE,
                "{content_type}",
            );
        }
    }
}

#[tokio::test]
async fn uses_custom_limit() {
    for limit in [LIMIT, DEFAULT_BODY_LIMIT + 1] {
        for filter in filters(limit) {
            for content_type in CONTENT_TYPES {
                let resp = post(&filter, content_type, limit).await;

                assert_eq!(resp.status(), StatusCode::OK, "{content_type}");

                let resp = post(&filter, content_type, limit + 1).await;

                assert_eq!(
                    resp.status(),
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "{content_type}",
                );
                assert_eq!(
                    resp.body(),
                    format!("Request body exceeds the limit of {limit} bytes").as_str(),
                );
            }
        }
    }
}

/// Tests with raw HTTP requests to a real server.
///
/// [`warp::test`] always sends the whole body with a `Content-Length` header. The tests of this
/// module send chunked bodies and incomplete bodies.
mod server {
    use std::time::Duration;

    use tokio::{
        io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
        net::{TcpListener, TcpStream},
        time,
    };
    use warp::http::StatusCode;

    use super::{GraphQLFilter, LIMIT, filters, padded_body};

    /// Maximum time to wait for the status line of a response.
    const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

    /// Sends the raw `request` to a server with the `filter` and returns the response status.
    ///
    /// The server must respond without waiting for more bytes of the `request`.
    async fn status(filter: GraphQLFilter, request: &str) -> StatusCode {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(warp::serve(filter).incoming(listener).run());

        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream.write_all(request.as_bytes()).await.unwrap();

        let mut status_line = String::new();
        time::timeout(
            RESPONSE_TIMEOUT,
            BufReader::new(stream).read_line(&mut status_line),
        )
        .await
        .expect("no response before the timeout")
        .unwrap();

        let code = status_line.split(' ').nth(1).unwrap();
        StatusCode::from_bytes(code.as_bytes()).unwrap()
    }

    #[tokio::test]
    async fn rejects_large_content_length() {
        // The request announces a body over the limit and sends no body.
        let request = format!(
            "POST / HTTP/1.1\r\n\
             Host: localhost\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             \r\n",
            LIMIT + 1,
        );

        for filter in filters(LIMIT) {
            assert_eq!(
                status(filter, &request).await,
                StatusCode::PAYLOAD_TOO_LARGE,
            );
        }
    }

    #[tokio::test]
    async fn rejects_large_chunked_body() {
        // The request sends one chunk over the limit and never ends the body.
        let request = format!(
            "POST / HTTP/1.1\r\n\
             Host: localhost\r\n\
             Content-Type: application/json\r\n\
             Transfer-Encoding: chunked\r\n\
             \r\n\
             {:x}\r\n{}\r\n",
            LIMIT + 1,
            " ".repeat(LIMIT + 1),
        );

        for filter in filters(LIMIT) {
            assert_eq!(
                status(filter, &request).await,
                StatusCode::PAYLOAD_TOO_LARGE,
            );
        }
    }

    #[tokio::test]
    async fn accepts_chunked_body_at_limit() {
        let body = padded_body("application/json", LIMIT);
        let (first, second) = body.split_at(LIMIT / 2);
        let request = format!(
            "POST / HTTP/1.1\r\n\
             Host: localhost\r\n\
             Content-Type: application/json\r\n\
             Transfer-Encoding: chunked\r\n\
             \r\n\
             {:x}\r\n{first}\r\n\
             {:x}\r\n{second}\r\n\
             0\r\n\r\n",
            first.len(),
            second.len(),
        );

        for filter in filters(LIMIT) {
            assert_eq!(status(filter, &request).await, StatusCode::OK);
        }
    }
}
