//! Common code generated parts, used by this crate.

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse_quote;

use crate::common::behavior;

/// Returns generated code implementing [`resolve::Resolvable`] trait for the
/// provided [`syn::Type`] with its [`syn::Generics`].
///
/// [`resolve::Resolvable`]: juniper::resolve::Resolvable
/// [0]: https://spec.graphql.org/October2021#sec-Interfaces
pub(crate) fn impl_resolvable(
    bh: &behavior::Type,
    (ty, generics): (syn::Type, syn::Generics),
) -> TokenStream {
    let (sv, generics) = mix_scalar_value(generics);
    let (impl_gens, _, where_clause) = generics.split_for_impl();

    quote! {
        #[automatically_derived]
        impl #impl_gens ::juniper::resolve::Resolvable<#sv, #bh>
         for #ty #where_clause
        {
            type Value = Self;

            fn into_value(self) -> ::juniper::FieldResult<Self, #sv> {
                ::juniper::FieldResult::Ok(self)
            }
        }
    }
}

/// Mixes a type info [`syn::GenericParam`] into the provided [`syn::Generics`]
/// and returns its [`syn::Ident`].
#[must_use]
pub(crate) fn mix_type_info(mut generics: syn::Generics) -> (syn::Ident, syn::Generics) {
    let ty = parse_quote! { __TypeInfo };
    generics.params.push(parse_quote! { #ty: ?Sized });
    (ty, generics)
}

/// Mixes a context [`syn::GenericParam`] into the provided [`syn::Generics`]
/// and returns its [`syn::Ident`].
pub(crate) fn mix_context(mut generics: syn::Generics) -> (syn::Ident, syn::Generics) {
    let ty = parse_quote! { __Context };
    generics.params.push(parse_quote! { #ty: ?Sized });
    (ty, generics)
}

/// Mixes a [`ScalarValue`] [`syn::GenericParam`] into the provided
/// [`syn::Generics`] and returns it.
///
/// [`ScalarValue`]: juniper::ScalarValue
pub(crate) fn mix_scalar_value(mut generics: syn::Generics) -> (syn::Ident, syn::Generics) {
    let sv = parse_quote! { __ScalarValue };
    generics.params.push(parse_quote! { #sv });
    (sv, generics)
}

/// Mixes an [`InputValue`]'s lifetime [`syn::GenericParam`] into the provided
/// [`syn::Generics`] and returns it.
///
/// [`InputValue`]: juniper::resolve::InputValue
#[must_use]
pub(crate) fn mix_input_lifetime(
    mut generics: syn::Generics,
    sv: impl ToTokens,
) -> (syn::GenericParam, syn::Generics) {
    let lt: syn::GenericParam = parse_quote! { '__inp };
    generics.params.push(lt.clone());
    generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { #sv: #lt });
    (lt, generics)
}

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
                Some((ctx, r)) => executor.replaced_context(ctx).resolve_with_ctx(info, &r),
                None => Ok(::juniper::Value::null()),
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
        Box::pin(::juniper::futures::FutureExt::then(fut, move |res #ty| async move {
            match ::juniper::IntoResolvable::into_resolvable(res, executor.context())? {
                Some((ctx, r)) => {
                    let subexec = executor.replaced_context(ctx);
                    subexec.resolve_with_ctx_async(info, &r).await
                },
                None => Ok(::juniper::Value::null()),
            }
        }))
    }
}
