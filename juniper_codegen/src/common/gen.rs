//! Common code generated parts, used by this crate.

use proc_macro2::TokenStream;
use quote::quote;

/// Generate the code resolving some [GraphQL type][1] in a synchronous manner.
///
/// Value of a [GraphQL type][1] should be stored in a `res` binding in the generated code, before
/// including this piece of code.
///
/// [1]: https://spec.graphql.org/October2021#sec-Types
pub(crate) fn sync_resolving_code() -> TokenStream {
    quote! {
        ::juniper::IntoResolvable::into_resolvable(res, executor.context())
            .and_then(|res| match res {
                ::core::option::Option::Some((ctx, r)) => {
                    executor.replaced_context(ctx).resolve_with_ctx(info, &r)
                }
                ::core::option::Option::None => {
                    ::core::result::Result::Ok(::juniper::Value::null())
                }
            })
    }
}

/// Generate the code resolving some [GraphQL type][1] in an asynchronous manner.
///
/// Value of a [GraphQL type][1] should be resolvable with `fut` binding representing a [`Future`]
/// in the generated code, before including this piece of code.
///
/// Optional `ty` argument may be used to annotate a concrete type of the resolving
/// [GraphQL type][1] (the [`Future::Output`]).
///
/// [`Future`]: std::future::Future
/// [`Future::Output`]: std::future::Future::Output
/// [1]: https://spec.graphql.org/October2021#sec-Types
pub(crate) fn async_resolving_code(ty: Option<&syn::Type>) -> TokenStream {
    let ty = ty.map(|t| quote! { : #t });

    quote! {
        ::std::boxed::Box::pin(::juniper::futures::FutureExt::then(fut, move |res #ty| async move {
            match ::juniper::IntoResolvable::into_resolvable(res, executor.context())? {
                ::core::option::Option::Some((ctx, r)) => {
                    let subexec = executor.replaced_context(ctx);
                    subexec.resolve_with_ctx_async(info, &r).await
                }
                ::core::option::Option::None => {
                    ::core::result::Result::Ok(::juniper::Value::null())
                }
            }
        }))
    }
}
