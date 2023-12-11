//! Code generation for `#[graphql_subscription]` macro.

use proc_macro2::{Span, TokenStream};

use crate::{
    common::parse,
    graphql_object::{attr::expand_on_impl, Attr},
};

use super::Subscription;

/// Expands `#[graphql_subscription]` macro into generated code.
pub fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemImpl>(body) {
        if ast.trait_.is_none() {
            let impl_attrs = parse::attr::unite(("graphql_subscription", &attr_args), &ast.attrs);
            ast.attrs = parse::attr::strip(["graphql_subscription", "graphql"], ast.attrs);
            return expand_on_impl::<Subscription>(
                Attr::from_attrs(["graphql_subscription", "graphql"], &impl_attrs)?,
                ast,
            );
        }
    }

    Err(syn::Error::new(
        Span::call_site(),
        "#[graphql_subscription] attribute is applicable to non-trait `impl` blocks only",
    ))
}
