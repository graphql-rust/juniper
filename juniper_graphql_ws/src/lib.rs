#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(missing_docs, warnings)]

#[cfg(not(any(feature = "graphql-transport-ws", feature = "graphql-ws")))]
compile_error!(
    r#"at least one feature must be enabled (either "graphql-transport-ws" or "graphql-ws")"#
);

#[cfg(feature = "graphql-transport-ws")]
pub mod graphql_transport_ws;
#[cfg(feature = "graphql-ws")]
pub mod graphql_ws;
mod schema;
mod server_message;
mod util;

use std::{
    convert::Infallible,
    error::Error,
    future::{self, Future},
    time::Duration,
};

use juniper::{ScalarValue, Variables};

pub use self::schema::{ArcSchema, Schema};

/// ConnectionConfig is used to configure the connection once the client sends the ConnectionInit
/// message.
#[derive(Clone, Copy, Debug)]
pub struct ConnectionConfig<CtxT> {
    /// Custom-provided [`juniper::Context`].
    pub context: CtxT,

    /// Maximum number of in-flight operations that a connection can have.
    ///
    /// If this number is exceeded, attempting to start more will result in an error.
    /// By default, there is no limit to in-flight operations.
    pub max_in_flight_operations: usize,

    /// Interval at which to send keep-alives.
    ///
    /// Specifying a [`Duration::ZERO`] will disable keep-alives.
    ///
    /// By default, keep-alives are sent every 15 seconds.
    pub keep_alive_interval: Duration,
}

impl<CtxT> ConnectionConfig<CtxT> {
    /// Constructs the configuration required for a connection to be accepted.
    pub fn new(context: CtxT) -> Self {
        Self {
            context,
            max_in_flight_operations: 0,
            keep_alive_interval: Duration::from_secs(15),
        }
    }

    /// Specifies the maximum number of in-flight operations that a connection can have.
    ///
    /// If this number is exceeded, attempting to start more will result in an error.
    /// By default, there is no limit to in-flight operations.
    #[must_use]
    pub fn with_max_in_flight_operations(mut self, max: usize) -> Self {
        self.max_in_flight_operations = max;
        self
    }

    /// Specifies the interval at which to send keep-alives.
    ///
    /// Specifying a [`Duration::ZERO`] will disable keep-alives.
    ///
    /// By default, keep-alives are sent every 15 seconds.
    #[must_use]
    pub fn with_keep_alive_interval(mut self, interval: Duration) -> Self {
        self.keep_alive_interval = interval;
        self
    }
}

impl<S: ScalarValue, CtxT: Unpin + Send + 'static> Init<S, CtxT> for ConnectionConfig<CtxT> {
    type Error = Infallible;
    type Future = future::Ready<Result<Self, Self::Error>>;

    fn init(self, _params: Variables<S>) -> Self::Future {
        future::ready(Ok(self))
    }
}

/// Init defines the requirements for types that can provide connection configurations when
/// ConnectionInit messages are received. Implementations are provided for `ConnectionConfig` and
/// closures that meet the requirements.
pub trait Init<S: ScalarValue, CtxT>: Unpin + 'static {
    /// The error that is returned on failure. The formatted error will be used as the contents of
    /// the "message" field sent back to the client.
    type Error: Error;

    /// The future configuration type.
    type Future: Future<Output = Result<ConnectionConfig<CtxT>, Self::Error>> + Send + 'static;

    /// Returns a future for the configuration to use.
    fn init(self, params: Variables<S>) -> Self::Future;
}

impl<F, S, CtxT, Fut, E> Init<S, CtxT> for F
where
    S: ScalarValue,
    F: FnOnce(Variables<S>) -> Fut + Unpin + 'static,
    Fut: Future<Output = Result<ConnectionConfig<CtxT>, E>> + Send + 'static,
    E: Error,
{
    type Error = E;
    type Future = Fut;

    fn init(self, params: Variables<S>) -> Fut {
        self(params)
    }
}
