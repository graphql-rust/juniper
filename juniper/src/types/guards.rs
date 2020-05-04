//!

use futures::future;

/// A guard for GraphQL paths/resources.
///
/// This guard is evaluated before the actual resolver logic is
/// executed. If the guard fails, then the underlying resolver logic
/// is not executed. Notice that guard can not access parameters. If
/// a potential guard requires explicit access to function specific
/// resources, then it must be implemented inside the resolver logic.
///
pub trait GraphQLGuard<S, CtxIn>
where
    S: crate::ScalarValue,
{
    /// Possible error which is thrown by the guard.
    ///
    /// This error may include indirect errors, e.g., failed database
    /// fetches.
    type Error: crate::IntoFieldError<S> + Send + Sync;

    /// Output context of the guard.
    ///
    /// A guard may create a different context which contains
    /// additional information for the underlying resolver. The new
    /// context must be compatible with the old context. Functions
    /// which does not require the new context should always have the
    /// opportunity to fallback.
    type CtxOut: Into<CtxIn> + Send + Sync + 'static;

    /// Protects a GraphQL path resource.
    fn protected<'a>(
        &'a self,
        ctx: &'a CtxIn,
    ) -> crate::BoxFuture<Result<&'a Self::CtxOut, Self::Error>>;
}

pub trait GuardExt<S: crate::ScalarValue, CtxIn>: GraphQLGuard<S, CtxIn> + Sized {
    /// Chains the current guard with another compatible guard.
    ///
    /// First the current guard is executed. If the execution of the
    /// first guard was successful, then the other guard is
    /// executed. In order to work with another guard, it must be
    /// ensured, that the error types are compatible such that the
    /// current error can be converted into the error of the other
    /// guard. Besides the errors, also the context must be
    /// compatible. The other guard must use the context of the
    /// current guard.
    fn and_then<'a, G: GraphQLGuard<S, Self::CtxOut>>(self, other: G) -> AndThen<Self, G> {
        AndThen {
            first: self,
            second: other,
        }
    }
}

impl<S: crate::ScalarValue, CtxIn, T: GraphQLGuard<S, CtxIn> + Sized> GuardExt<S, CtxIn> for T {}

pub struct AndThen<A, B> {
    first: A,
    second: B,
}

impl<A, B, CtxIn, S> GraphQLGuard<S, CtxIn> for AndThen<A, B>
where
    S: crate::ScalarValue,
    CtxIn: std::convert::From<<B as GraphQLGuard<S, <A as GraphQLGuard<S, CtxIn>>::CtxOut>>::CtxOut>
        + Send
        + Sync,
    A: GraphQLGuard<S, CtxIn> + Send + Sync,
    B: GraphQLGuard<S, <A as GraphQLGuard<S, CtxIn>>::CtxOut> + Send + Sync,
    <A as GraphQLGuard<S, CtxIn>>::Error:
        Into<<B as GraphQLGuard<S, <A as GraphQLGuard<S, CtxIn>>::CtxOut>>::Error>,
{
    type Error = <B as GraphQLGuard<S, <A as GraphQLGuard<S, CtxIn>>::CtxOut>>::Error;

    type CtxOut = <B as GraphQLGuard<S, <A as GraphQLGuard<S, CtxIn>>::CtxOut>>::CtxOut;

    fn protected<'a>(
        &'a self,
        ctx: &'a CtxIn,
    ) -> crate::BoxFuture<Result<&'a Self::CtxOut, Self::Error>> {
        let f = async move {
            match self.first.protected(&ctx).await {
                Ok(ctx) => self.second.protected(&ctx).await,
                Err(err) => Err(err.into()),
            }
        };

        future::FutureExt::boxed(f)
    }
}
