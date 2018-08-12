extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate juniper;

use futures_cpupool::{CpuFuture, CpuPool};

use futures::future::IntoFuture;
use futures::Future;
use hyper::service::{NewService, Service};
use hyper::Request;
use hyper::{Body, Response, Server};
use juniper::GraphQLType;
use juniper::RootNode;

use hyper::body::Payload;
use std::error::Error as StdError;

pub struct GraphQLHandler<'a, CtxFactory, CtxRes, Query, Mutation, CtxT>
where
    CtxFactory: Fn(&mut Request<Body>) -> CtxRes + Send + Sync,
    CtxRes: IntoFuture<Item = CtxT>,
    CtxRes::Error: Into<Box<StdError + Send + Sync>>,
    Query: GraphQLType<Context = CtxT> + Send + Sync,
    Mutation: GraphQLType<Context = CtxT> + Send + Sync,
{
    pool: CpuPool,
    context_factory: CtxFactory,
    root_node: RootNode<'a, Query, Mutation>,
}

impl<'a, CtxFactory, CtxRes, Query, Mutation, CtxT>
    GraphQLHandler<'a, CtxFactory, CtxRes, Query, Mutation, CtxT>
where
    CtxFactory: Fn(&mut Request<Body>) -> CtxRes + Send + Sync,
    CtxRes: IntoFuture<Item = CtxT>,
    CtxRes::Error: Into<Box<StdError + Send + Sync>>,
    Query: GraphQLType<Context = CtxT, TypeInfo = ()> + Send + Sync,
    Mutation: GraphQLType<Context = CtxT, TypeInfo = ()> + Send + Sync,
{
    pub fn new(
        pool: CpuPool,
        context_factory: CtxFactory,
        root_node: RootNode<'a, Query, Mutation>,
    ) -> Self {
        GraphQLHandler::new_with_info(pool, context_factory, query, (), mutation, ())
    }
}

impl<'a, CtxFactory, CtxRes, Query, Mutation, CtxT> Service
    for GraphQLHandler<'a, CtxFactory, CtxRes, Query, Mutation, CtxT>
where
    CtxT: Send,
    CtxFactory: Fn(&mut Request<Body>) -> CtxRes + Send + Sync,
    CtxRes: IntoFuture<Item = CtxT> + Send,
    CtxRes::Error: Into<Box<StdError + Send + Sync>>,
    CtxRes::Future: 'static + Send,
    Query: GraphQLType<Context = CtxT> + Send + Sync,
    Mutation: GraphQLType<Context = CtxT> + Send + Sync,
{
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Box<std::error::Error + Send + Sync>;
    type Future = Box<Future<Item = Response<Self::ResBody>, Error = Self::Error> + Send>;

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        Box::new(
            (self.context_factory)(&mut req)
                .into_future()
                .map_err(|err| err.into())
                .map(|res: CtxT| Response::new(Body::from("YO!"))),
        )
    }
}

#[cfg(test)]
mod tests {}
