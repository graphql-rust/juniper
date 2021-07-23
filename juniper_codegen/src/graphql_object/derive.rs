//! Code generation for `#[derive(GraphQLObject)]` macro.

use proc_macro2::TokenStream;
use proc_macro_error::ResultExt as _;
use quote::ToTokens;
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned as _, Data, Fields};

use crate::{result::GraphQLScope, util::span_container::SpanContainer};

use super::{Definition, ObjectMeta};

/// [`GraphQLScope`] of errors for `#[derive(GraphQLObject)]` macro.
const ERR: GraphQLScope = GraphQLScope::ObjectDerive;

/// Expands `#[derive(GraphQLObject)]` macro into generated code.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input).unwrap_or_abort();

    match &ast.data {
        Data::Struct(_) => expand_struct(ast),
        _ => Err(ERR.custom_error(ast.span(), "can only be derived forstructs")),
    }
    .map(ToTokens::into_token_stream)
}

/// Expands into generated code a `#[derive(GraphQLObject)]` macro placed on a Rust struct.
fn expand_struct(ast: syn::DeriveInput) -> syn::Result<Definition> {
    let meta = ObjectMeta::from_attrs("graphql", &ast.attrs)?;

    let struct_span = ast.span();
    let struct_ident = ast.ident;

    let name = meta
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| struct_ident.unraw().to_string());
    if !meta.is_internal && name.starts_with("__") {
        ERR.no_double_underscore(
            meta.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| struct_ident.span()),
        );
    }

    Ok(Definition {
        name,
        ty: parse_quote! { #struct_ident },
        generics: ast.generics,
        description: meta.description.map(SpanContainer::into_inner),
        context: meta.context.map(SpanContainer::into_inner),
        scalar: meta.scalar.map(SpanContainer::into_inner),
    })
}
