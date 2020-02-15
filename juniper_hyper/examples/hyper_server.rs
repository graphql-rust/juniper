extern crate juniper;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};

use std::sync::Arc;
use juniper::{tests::{model::Database, schema::Query}, EmptyMutation, RootNode, ScalarValue, GraphQLType};


type GenericError = Box<dyn std::error::Error + Send + Sync>;
type HyperResult<T> = std::result::Result<T, GenericError>;


async fn response_examples<QueryT, MutationT, S, CtxT>(
    req: Request<Body>,
    db: Arc<CtxT>,
    root_node: Arc<RootNode<'static, QueryT, MutationT, S>>,
) -> HyperResult<Response<Body>>
    where
        S: ScalarValue + Send + Sync + 'static,
        CtxT: Send + Sync + 'static,
        QueryT: GraphQLType<S, Context=CtxT> + Send + Sync + 'static,
        MutationT: GraphQLType<S, Context=CtxT> + Send + Sync + 'static,
        QueryT::TypeInfo: Send + Sync,
        MutationT::TypeInfo: Send + Sync,
{
    let root_node = root_node.clone();
    let db = db.clone();
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => Ok(juniper_hyper::graphiql("/graphql").await?),
        (&Method::GET, "/graphql") | (&Method::POST, "/graphql") => {
            Ok(juniper_hyper::graphql(root_node, db, req).await?)
        }
        _ => {
            // Return 404 not found response.
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not found"))
                .unwrap())
        }
    }
}


#[tokio::main]
async fn main() -> HyperResult<()> {
    let addr = ([127, 0, 0, 1], 3000).into();
    let db = Arc::new(Database::new());
    let root_node = Arc::new(RootNode::new(Query, EmptyMutation::<Database>::new()));

    let new_service = make_service_fn(move |_| {
        let root_node = root_node.clone();
        let db = db.clone();
        async {
            Ok::<_, GenericError>(service_fn(move |req| {
                // Clone again to ensure that client outlives this closure.
                response_examples(req, db.clone(), root_node.clone())

            }))
        }
    });

    let server = Server::bind(&addr).serve(new_service);

    println!("Server listening on http://{}", addr);

    server.await?;

    Ok(())
}
