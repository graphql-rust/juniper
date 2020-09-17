use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn sync_resolving_code() -> TokenStream {
    quote! {
        ::juniper::IntoResolvable::into(res, executor.context())
            .and_then(|res| match res {
                Some((ctx, r)) => executor.replaced_context(ctx).resolve_with_ctx(info, &r),
                None => Ok(::juniper::Value::null()),
            })
    }
}

pub(crate) fn async_resolving_code(ty: Option<&syn::Type>) -> TokenStream {
    let ty = ty.map(|t| quote! { : #t });

    quote! {
        Box::pin(::juniper::futures::FutureExt::then(fut, move |res #ty| async move {
            match ::juniper::IntoResolvable::into(res, executor.context())? {
                Some((ctx, r)) => {
                    let subexec = executor.replaced_context(ctx);
                    subexec.resolve_with_ctx_async(info, &r).await
                },
                None => Ok(::juniper::Value::null()),
            }
        }))
    }
}
