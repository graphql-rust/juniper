//!

use futures::future;

/// A replaceable context.
///
/// The underlying item is either owned or referenced. Compared to the
/// built-in Cow<'a, T>, this does not allow writes. Therefore, it is
/// only possible to get a reference of T. In the terms of Juniper
/// this allows to replace the context within a guard.
pub enum MaybeOwned<'a, T> {
    /// Owns the underlying data.
    ///
    // Indicates that the value has been replaced.
    Owned(T),
    /// References to the data.
    ///
    // Indicates that the value has been take by someone else.
    Reference(&'a T),
}

impl<'a, T> AsRef<T> for MaybeOwned<'a, T> {
    fn as_ref(&self) -> &T {
        match self {
            MaybeOwned::Owned(ref val) => val,
            MaybeOwned::Reference(ref val) => val,
        }
    }
}

impl<'a, T> From<T> for MaybeOwned<'a, T> {
    fn from(val: T) -> Self {
        MaybeOwned::Owned(val)
    }
}

impl<'a, T> From<&'a T> for MaybeOwned<'a, T> {
    fn from(val: &'a T) -> Self {
        MaybeOwned::Reference(val)
    }
}

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
    type CtxOut: Send + Sync;

    /// Protects a GraphQL path resource.
    fn protected<'a>(
        &'a self,
        ctx: MaybeOwned<'a, CtxIn>,
    ) -> crate::BoxFuture<Result<MaybeOwned<'a, Self::CtxOut>, Self::Error>>;
}

/// Additional functionality on-top of `GraphQLGuard`
///
/// Provides helper functions for guards.
pub trait GraphQLGuardExt<S: crate::ScalarValue, CtxIn>: GraphQLGuard<S, CtxIn> + Sized {
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
    fn and_then<G: GraphQLGuard<S, Self::CtxOut>>(self, other: G) -> AndThen<Self, G> {
        AndThen {
            first: self,
            second: other,
        }
    }
}

impl<S: crate::ScalarValue, CtxIn, T: GraphQLGuard<S, CtxIn> + Sized> GraphQLGuardExt<S, CtxIn>
    for T
{
}

/// Chaining two guards.
///
/// This is a product of the `and_then` operation for guards.
pub struct AndThen<A, B> {
    first: A,
    second: B,
}

impl<A, B, CtxIn, S> GraphQLGuard<S, CtxIn> for AndThen<A, B>
where
    S: crate::ScalarValue,
    CtxIn: std::convert::From<<B as GraphQLGuard<S, <A as GraphQLGuard<S, CtxIn>>::CtxOut>>::CtxOut>
        + Send
        + Sync
        + 'static,
    A: GraphQLGuard<S, CtxIn> + Send + Sync,
    B: GraphQLGuard<S, <A as GraphQLGuard<S, CtxIn>>::CtxOut> + Send + Sync + 'static,
    <A as GraphQLGuard<S, CtxIn>>::CtxOut: 'static,
    <A as GraphQLGuard<S, CtxIn>>::Error:
        Into<<B as GraphQLGuard<S, <A as GraphQLGuard<S, CtxIn>>::CtxOut>>::Error>,
{
    type Error = <B as GraphQLGuard<S, <A as GraphQLGuard<S, CtxIn>>::CtxOut>>::Error;

    type CtxOut = <B as GraphQLGuard<S, <A as GraphQLGuard<S, CtxIn>>::CtxOut>>::CtxOut;

    fn protected<'a>(
        &'a self,
        ctx: MaybeOwned<'a, CtxIn>,
    ) -> crate::BoxFuture<Result<MaybeOwned<'a, Self::CtxOut>, Self::Error>> {
        let f = async move {
            match self.first.protected(ctx).await {
                Ok(new_ctx) => self.second.protected(new_ctx).await,
                Err(err) => Err(err.into()),
            }
        };

        future::FutureExt::boxed(f)
    }
}
