/*!

# juniper_actix

This repository contains the [actix][actix] web server integration for
[Juniper][Juniper], a [GraphQL][GraphQL] implementation for Rust, its inspired and some parts are copied from [juniper_warp][juniper_warp]

## Documentation

For documentation, including guides and examples, check out [Juniper][Juniper].

A basic usage example can also be found in the [API documentation][documentation].

## Examples

Check [examples/actix_server][example] for example code of a working actix
server with GraphQL handlers.

## Links

* [Juniper][Juniper]
* [API Reference][documentation]
* [actix][actix]

## License

This project is under the BSD-2 license.

Check the LICENSE file for details.

[actix]: https://github.com/actix/actix-web
[Juniper]: https://github.com/graphql-rust/juniper
[GraphQL]: http://graphql.org
[documentation]: https://docs.rs/juniper_actix
[example]: https://github.com/graphql-rust/juniper/blob/master/juniper_actix/examples/actix_server.rs
[juniper_warp]: https://github.com/graphql-rust/juniper/juniper_warp
*/

#![deny(missing_docs)]
#![deny(warnings)]
#![doc(html_root_url = "https://docs.rs/juniper_actix/0.1.0")]

// use futures::{FutureExt as _};
use actix_web::{
    error::{ErrorBadRequest, ErrorMethodNotAllowed, ErrorUnsupportedMediaType},
    http::{header::CONTENT_TYPE, Method},
    web, Error, FromRequest, HttpRequest, HttpResponse,
};
use juniper::{
    http::{
        graphiql::graphiql_source, playground::playground_source, GraphQLBatchRequest,
        GraphQLRequest,
    },
    ScalarValue,
};
use serde::Deserialize;

#[serde(deny_unknown_fields)]
#[derive(Deserialize, Clone, PartialEq, Debug)]
struct GetGraphQLRequest {
    query: String,
    #[serde(rename = "operationName")]
    operation_name: Option<String>,
    variables: Option<String>,
}

impl<S> From<GetGraphQLRequest> for GraphQLRequest<S>
where
    S: ScalarValue,
{
    fn from(get_req: GetGraphQLRequest) -> Self {
        let GetGraphQLRequest {
            query,
            operation_name,
            variables,
        } = get_req;
        let variables = match variables {
            Some(variables) => Some(serde_json::from_str(&variables).unwrap()),
            None => None,
        };
        Self::new(query, operation_name, variables)
    }
}

/// Actix Web GraphQL Handler for GET and POST requests
pub async fn graphql_handler<Query, Mutation, Subscription, Context, S>(
    schema: &juniper::RootNode<'static, Query, Mutation, Subscription, S>,
    context: &Context,
    req: HttpRequest,
    payload: actix_web::web::Payload,
) -> Result<HttpResponse, Error>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
{
    match *req.method() {
        Method::POST => post_graphql_handler(schema, context, req, payload).await,
        Method::GET => get_graphql_handler(schema, context, req).await,
        _ => Err(ErrorMethodNotAllowed(
            "GraphQL requests can only be sent with GET or POST",
        )),
    }
}
/// Actix GraphQL Handler for GET requests
pub async fn get_graphql_handler<Query, Mutation, Subscription, Context, S>(
    schema: &juniper::RootNode<'static, Query, Mutation, Subscription, S>,
    context: &Context,
    req: HttpRequest,
) -> Result<HttpResponse, Error>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
{
    let get_req = web::Query::<GetGraphQLRequest>::from_query(req.query_string())?;
    let req = GraphQLRequest::from(get_req.into_inner());
    let gql_response = req.execute(schema, context).await;
    let body_response = serde_json::to_string(&gql_response)?;
    let response = match gql_response.is_ok() {
        true => HttpResponse::Ok()
            .content_type("application/json")
            .body(body_response),
        false => HttpResponse::BadRequest()
            .content_type("application/json")
            .body(body_response),
    };
    Ok(response)
}

/// Actix GraphQL Handler for POST requests
pub async fn post_graphql_handler<Query, Mutation, Subscription, Context, S>(
    schema: &juniper::RootNode<'static, Query, Mutation, Subscription, S>,
    context: &Context,
    req: HttpRequest,
    payload: actix_web::web::Payload,
) -> Result<HttpResponse, Error>
where
    S: ScalarValue + Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Query: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Query::TypeInfo: Send + Sync,
    Mutation: juniper::GraphQLTypeAsync<S, Context = Context> + Send + Sync + 'static,
    Mutation::TypeInfo: Send + Sync,
    Subscription: juniper::GraphQLSubscriptionType<S, Context = Context> + Send + Sync + 'static,
    Subscription::TypeInfo: Send + Sync,
{
    let content_type_header = req
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|hv| hv.to_str().ok());
    let req = match content_type_header {
        Some("application/json") | Some("application/graphql") => {
            let body_string = String::from_request(&req, &mut payload.into_inner()).await;
            let body_string = body_string?;
            match serde_json::from_str::<GraphQLBatchRequest<S>>(&body_string) {
                Ok(req) => Ok(req),
                Err(err) => Err(ErrorBadRequest(err)),
            }
        }
        _ => Err(ErrorUnsupportedMediaType(
            "GraphQL requests should have content type `application/json` or `application/graphql`",
        )),
    }?;
    let gql_batch_response = req.execute(schema, context).await;
    let gql_response = serde_json::to_string(&gql_batch_response)?;
    let mut response = match gql_batch_response.is_ok() {
        true => HttpResponse::Ok(),
        false => HttpResponse::BadRequest(),
    };
    Ok(response.content_type("application/json").body(gql_response))
}

/// Create a handler that replies with an HTML page containing GraphiQL. This does not handle routing, so you can mount it on any endpoint
///
/// For example:
///
/// ```
/// # extern crate actix;
/// # extern crate juniper_actix;
/// #
/// # use juniper_actix::graphiql_handler;
/// # use actix_web::{web, App};
///
/// let app = App::new()
///          .route("/", web::get().to(|| graphiql_handler("/graphql", Some("/graphql/subscriptions"))));
/// ```
#[allow(dead_code)]
pub async fn graphiql_handler(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: Option<&'static str>,
) -> Result<HttpResponse, Error> {
    let html = graphiql_source(graphql_endpoint_url, subscriptions_endpoint_url);
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

/// Create a handler that replies with an HTML page containing GraphQL Playground. This does not handle routing, so you cant mount it on any endpoint.
pub async fn playground_handler(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: Option<&'static str>,
) -> Result<HttpResponse, Error> {
    let html = playground_source(graphql_endpoint_url, subscriptions_endpoint_url);
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{dev::ServiceResponse, http, http::header::CONTENT_TYPE, test, App};
    use futures::StreamExt;
    use juniper::{
        http::tests::{run_http_test_suite, HTTPIntegration, TestResponse},
        tests::{model::Database, schema::Query},
        EmptyMutation, EmptySubscription, RootNode,
    };

    type Schema =
        juniper::RootNode<'static, Query, EmptyMutation<Database>, EmptySubscription<Database>>;

    async fn take_response_body_string(resp: &mut ServiceResponse) -> String {
        let (response_body, ..) = resp
            .take_body()
            .map(|body_out| body_out.unwrap().to_vec())
            .into_future()
            .await;
        match response_body {
            Some(response_body) => String::from_utf8(response_body).unwrap(),
            None => String::from(""),
        }
    }

    async fn index(
        req: HttpRequest,
        payload: actix_web::web::Payload,
        schema: web::Data<Schema>,
    ) -> Result<HttpResponse, Error> {
        let context = Database::new();
        graphql_handler(&schema, &context, req, payload).await
    }

    #[actix_rt::test]
    async fn graphiql_response_does_not_panic() {
        let result = graphiql_handler("/abcd", None).await;
        assert!(result.is_ok())
    }

    #[actix_rt::test]
    async fn graphiql_endpoint_matches() {
        async fn graphql_handler() -> Result<HttpResponse, Error> {
            graphiql_handler("/abcd", None).await
        }
        let mut app =
            test::init_service(App::new().route("/", web::get().to(graphql_handler))).await;
        let req = test::TestRequest::get()
            .uri("/")
            .header("accept", "text/html")
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_rt::test]
    async fn graphiql_endpoint_returns_graphiql_source() {
        async fn graphql_handler() -> Result<HttpResponse, Error> {
            graphiql_handler("/dogs-api/graphql", Some("/dogs-api/subscriptions")).await
        }
        let mut app =
            test::init_service(App::new().route("/", web::get().to(graphql_handler))).await;
        let req = test::TestRequest::get()
            .uri("/")
            .header("accept", "text/html")
            .to_request();

        let mut resp = test::call_service(&mut app, req).await;
        let body = take_response_body_string(&mut resp).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap(),
            "text/html; charset=utf-8"
        );
        assert!(body.contains("<script>var GRAPHQL_URL = '/dogs-api/graphql';</script>"));
        assert!(body.contains(
            "<script>var GRAPHQL_SUBSCRIPTIONS_URL = '/dogs-api/subscriptions';</script>"
        ))
    }

    #[actix_rt::test]
    async fn playground_endpoint_matches() {
        async fn graphql_handler() -> Result<HttpResponse, Error> {
            playground_handler("/abcd", None).await
        }
        let mut app =
            test::init_service(App::new().route("/", web::get().to(graphql_handler))).await;
        let req = test::TestRequest::get()
            .uri("/")
            .header("accept", "text/html")
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_rt::test]
    async fn playground_endpoint_returns_playground_source() {
        async fn graphql_handler() -> Result<HttpResponse, Error> {
            playground_handler("/dogs-api/graphql", Some("/dogs-api/subscriptions")).await
        }
        let mut app =
            test::init_service(App::new().route("/", web::get().to(graphql_handler))).await;
        let req = test::TestRequest::get()
            .uri("/")
            .header("accept", "text/html")
            .to_request();

        let mut resp = test::call_service(&mut app, req).await;
        let body = take_response_body_string(&mut resp).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap(),
            "text/html; charset=utf-8"
        );
        assert!(body.contains("GraphQLPlayground.init(root, { endpoint: '/dogs-api/graphql', subscriptionEndpoint: '/dogs-api/subscriptions' })"));
    }

    #[actix_rt::test]
    async fn graphql_post_works_json_post() {
        let schema: Schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );

        let req = test::TestRequest::post()
            .header("content-type", "application/json")
            .set_payload(
                r##"{ "variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }" }"##,
            )
            .uri("/")
            .to_request();

        let mut app =
            test::init_service(App::new().data(schema).route("/", web::post().to(index))).await;

        let mut resp = test::call_service(&mut app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            take_response_body_string(&mut resp).await,
            r#"{"data":{"hero":{"name":"R2-D2"}}}"#
        );
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json",
        );
    }

    #[actix_rt::test]
    async fn graphql_get_works() {
        let schema: Schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );

        let req = test::TestRequest::get()
            .header("content-type", "application/json")
            .uri("/?query=%7B%20hero%28episode%3A%20NEW_HOPE%29%20%7B%20name%20%7D%20%7D&variables=null")
            .to_request();

        let mut app =
            test::init_service(App::new().data(schema).route("/", web::get().to(index))).await;

        let mut resp = test::call_service(&mut app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            take_response_body_string(&mut resp).await,
            r#"{"data":{"hero":{"name":"R2-D2"}}}"#
        );
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json",
        );
    }

    #[actix_rt::test]
    async fn batch_request_works() {
        use juniper::{
            tests::{model::Database, schema::Query},
            EmptyMutation, EmptySubscription, RootNode,
        };

        let schema: Schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );

        let req = test::TestRequest::post()
            .header("content-type", "application/json")
            .set_payload(
                r##"[
                     { "variables": null, "query": "{ hero(episode: NEW_HOPE) { name } }" },
                     { "variables": null, "query": "{ hero(episode: EMPIRE) { id name } }" }
                 ]"##,
            )
            .uri("/")
            .to_request();

        let mut app =
            test::init_service(App::new().data(schema).route("/", web::post().to(index))).await;

        let mut resp = test::call_service(&mut app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            take_response_body_string(&mut resp).await,
            r#"[{"data":{"hero":{"name":"R2-D2"}}},{"data":{"hero":{"id":"1000","name":"Luke Skywalker"}}}]"#
        );
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json",
        );
    }

    #[test]
    fn batch_request_deserialization_can_fail() {
        let json = r#"blah"#;
        let result: Result<GraphQLBatchRequest, _> = serde_json::from_str(json);

        assert!(result.is_err());
    }

    pub struct TestActixWebIntegration {}

    impl HTTPIntegration for TestActixWebIntegration {
        fn get(&self, url: &str) -> TestResponse {
            let url = url.to_string();
            actix_rt::System::new("get_request").block_on(async move {
                let schema: Schema = RootNode::new(
                    Query,
                    EmptyMutation::<Database>::new(),
                    EmptySubscription::<Database>::new(),
                );
                let req = test::TestRequest::get()
                    .header("content-type", "application/json")
                    .uri(&url.clone())
                    .to_request();

                let mut app =
                    test::init_service(App::new().data(schema).route("/", web::get().to(index)))
                        .await;

                let resp = test::call_service(&mut app, req).await;
                let test_response = make_test_response(resp).await;
                test_response
            })
        }

        fn post(&self, url: &str, body: &str) -> TestResponse {
            let url = url.to_string();
            let body = body.to_string();
            actix_rt::System::new("post_request").block_on(async move {
                let schema: Schema = RootNode::new(
                    Query,
                    EmptyMutation::<Database>::new(),
                    EmptySubscription::<Database>::new(),
                );

                let req = test::TestRequest::post()
                    .header("content-type", "application/json")
                    .set_payload(body)
                    .uri(&url.clone())
                    .to_request();

                let mut app =
                    test::init_service(App::new().data(schema).route("/", web::post().to(index)))
                        .await;

                let resp = test::call_service(&mut app, req).await;
                let test_response = make_test_response(resp).await;
                test_response
            })
        }
    }

    async fn make_test_response(mut response: ServiceResponse) -> TestResponse {
        let body = take_response_body_string(&mut response).await;
        let status_code = response.status().as_u16();
        let content_type = response.headers().get(CONTENT_TYPE).unwrap();
        TestResponse {
            status_code: status_code as i32,
            body: Some(body),
            content_type: content_type.to_str().unwrap().to_string(),
        }
    }

    #[test]
    fn test_actix_web_integration() {
        run_http_test_suite(&TestActixWebIntegration {});
    }
}
