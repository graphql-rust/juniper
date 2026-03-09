#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(any(doc, test), doc = include_str!("../README.md"))]
#![cfg_attr(not(any(doc, test)), doc = env!("CARGO_PKG_NAME"))]

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

use std::{convert::Infallible, error::Error, future, time::Duration};

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

    /// Keep-alive configuration.
    pub keep_alive: KeepAliveConfig,
}

impl<CtxT> ConnectionConfig<CtxT> {
    /// Constructs the configuration required for a connection to be accepted.
    pub fn new(context: CtxT) -> Self {
        Self {
            context,
            max_in_flight_operations: 0,
            keep_alive: KeepAliveConfig::default(),
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
    /// Also, sets a keep-alive timeout to the provided [`Duration`].
    ///
    /// By default, keep-alives are sent every 15 seconds.
    #[must_use]
    pub fn with_keep_alive_interval(mut self, interval: Duration) -> Self {
        self.keep_alive.interval = interval;
        #[cfg(feature = "graphql-transport-ws")]
        {
            self.keep_alive.timeout = interval;
        }
        self
    }

    #[cfg(feature = "graphql-transport-ws")]
    /// Specifies the timeout for waiting a keep-alive response from clients after sending them a
    /// keep-alive message.
    ///
    /// Once the timeout is hit, the connection is closed by the server.
    ///
    /// Specifying a [`Duration::ZERO`] disables timeout checking.
    ///
    /// Applicable only for the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new],
    /// and does nothing for the [legacy `graphql-ws` GraphQL over WebSocket Protocol][old].
    ///
    /// By default, timeout equals to the [`KeepAliveConfig::interval`].
    ///
    /// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
    /// [old]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md
    #[must_use]
    pub fn with_keep_alive_timeout(mut self, timeout: Duration) -> Self {
        self.keep_alive.timeout = timeout;
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

/// Config for keeping a connection alive.
#[derive(Clone, Copy, Debug)]
pub struct KeepAliveConfig {
    /// Interval at which to send keep-alives.
    ///
    /// Specifying a [`Duration::ZERO`] disables keep-alives.
    ///
    /// By default, keep-alives are sent every 15 seconds.
    pub interval: Duration,

    #[cfg(feature = "graphql-transport-ws")]
    /// Timeout for waiting a keep-alive response from clients after sending them a keep-alive
    /// message.
    ///
    /// Once the timeout is hit, the connection is closed by the server.
    ///
    /// Specifying a [`Duration::ZERO`] disables timeout checking.
    ///
    /// Applicable only for the [new `graphql-transport-ws` GraphQL over WebSocket Protocol][new],
    /// and does nothing for the [legacy `graphql-ws` GraphQL over WebSocket Protocol][old].
    ///
    /// By default, timeout equals to the [`interval`].
    ///
    /// [`interval`]: Self::interval
    /// [new]: https://github.com/enisdenjo/graphql-ws/blob/v5.14.0/PROTOCOL.md
    /// [old]: https://github.com/apollographql/subscriptions-transport-ws/blob/v0.11.0/PROTOCOL.md
    pub timeout: Duration,
}

impl Default for KeepAliveConfig {
    fn default() -> Self {
        let interval = Duration::from_secs(15);
        Self {
            interval,
            #[cfg(feature = "graphql-transport-ws")]
            timeout: interval,
        }
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
