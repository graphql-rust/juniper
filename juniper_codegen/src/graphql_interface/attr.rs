//! Code generation for `#[graphql_interface]` macro.

use proc_macro2::{Span, TokenStream};

use crate::{
    result::GraphQLScope,
    util::{strip_attr, unite_attrs},
};

/// [`GraphQLScope`] of errors for `#[graphql_interface]` macro.
const ERR: GraphQLScope = GraphQLScope::InterfaceAttr;

/// Expands `#[graphql_interface]` macro into generated code.
pub fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemTrait>(body.clone()) {
        let trait_attrs = unite_attrs(("graphql_interface", &attr_args), &ast.attrs);
        ast.attrs = strip_attr("graphql_interface", ast.attrs);
        expand_on_trait(trait_attrs, ast)
    } else if let Ok(mut ast) = syn::parse2::<syn::ItemImpl>(body) {
        let impl_attrs = unite_attrs(("graphql_interface", &attr_args), &ast.attrs);
        ast.attrs = strip_attr("graphql_interface", ast.attrs);
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
    todo!()
}

/// Expands `#[graphql_interface]` macro placed on trait implementation block.
pub fn expand_on_impl(attrs: Vec<syn::Attribute>, ast: syn::ItemImpl) -> syn::Result<TokenStream> {
    todo!()
}
