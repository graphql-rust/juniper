//! Code generation for `#[graphql_interface]` macro.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned as _};

use crate::{
    result::GraphQLScope,
    util::{span_container::SpanContainer, strip_attrs, unite_attrs},
};

use super::{InterfaceDefinition, InterfaceMeta};

/// [`GraphQLScope`] of errors for `#[graphql_interface]` macro.
const ERR: GraphQLScope = GraphQLScope::InterfaceAttr;

/// Expands `#[graphql_interface]` macro into generated code.
pub fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemTrait>(body.clone()) {
        let trait_attrs = unite_attrs(("graphql_interface", &attr_args), &ast.attrs);
        ast.attrs = strip_attrs("graphql_interface", ast.attrs);
        expand_on_trait(trait_attrs, ast)
    } else if let Ok(mut ast) = syn::parse2::<syn::ItemImpl>(body) {
        let impl_attrs = unite_attrs(("graphql_interface", &attr_args), &ast.attrs);
        ast.attrs = strip_attrs("graphql_interface", ast.attrs);
        expand_on_impl(impl_attrs, ast)
    } else {
        Err(syn::Error::new(
            Span::call_site(),
            "#[graphql_interface] attribute is applicable to trait definitions and trait \
             implementations only",
        ))
    }
}

/// Expands `#[graphql_interface]` macro placed on trait definition.
pub fn expand_on_trait(
    attrs: Vec<syn::Attribute>,
    ast: syn::ItemTrait,
) -> syn::Result<TokenStream> {
    let meta = InterfaceMeta::from_attrs("graphql_interface", &attrs)?;

    let trait_span = ast.span();
    let trait_ident = &ast.ident;

    let name = meta
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| trait_ident.unraw().to_string());

    let context = meta.context.map(SpanContainer::into_inner);
    //.or_else(|| variants.iter().find_map(|v| v.context_ty.as_ref()).cloned());

    let generated_code = InterfaceDefinition {
        name,
        ty: parse_quote! { #trait_ident },
        is_trait_object: true,
        description: meta.description.map(SpanContainer::into_inner),
        context,
        scalar: meta.scalar.map(SpanContainer::into_inner),
        generics: ast.generics.clone(),
        implementers: vec![], // TODO
    };

    Ok(quote! {
        #ast

        #generated_code
    })
}

/// Expands `#[graphql_interface]` macro placed on trait implementation block.
pub fn expand_on_impl(attrs: Vec<syn::Attribute>, ast: syn::ItemImpl) -> syn::Result<TokenStream> {
    todo!()
}
