use std::{convert::Infallible, env, error::Error, net::SocketAddr, sync::Arc};

use hyper::{server::conn::http1, service::service_fn, Method, Response, StatusCode};
use hyper_util::rt::TokioIo;
use juniper::{
    tests::fixtures::starwars::schema::{Database, Query},
    EmptyMutation, EmptySubscription, RootNode,
};
use juniper_hyper::{graphiql, graphql, playground};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();

    let db = Arc::new(Database::new());
    let root_node = Arc::new(RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    ));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    log::info!("Listening on http://{addr}");
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        let root_node = root_node.clone();
        let db = db.clone();

        tokio::spawn(async move {
            let root_node = root_node.clone();
            let db = db.clone();

            if let Err(e) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| {
                        let root_node = root_node.clone();
                        let db = db.clone();
                        async {
                            Ok::<_, Infallible>(match (req.method(), req.uri().path()) {
                                (&Method::GET, "/graphql") | (&Method::POST, "/graphql") => {
                                    graphql(root_node, db, req).await
                                }
                                (&Method::GET, "/graphiql") => graphiql("/graphql", None).await,
                                (&Method::GET, "/playground") => playground("/graphql", None).await,
                                _ => {
                                    let mut resp = Response::new(String::new());
                                    *resp.status_mut() = StatusCode::NOT_FOUND;
                                    resp
                                }
                            })
                        }
                    }),
                )
                .await
            {
                log::error!("Error serving connection: {e}");
            }
        });
    }
}
