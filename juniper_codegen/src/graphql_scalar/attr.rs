//! Code generation for `#[graphql_scalar]` macro.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, spanned::Spanned};

use crate::common::{diagnostic, parse, scalar, SpanContainer};

use super::{derive::parse_derived_methods, Attr, Definition, Methods, ParseToken, TypeOrIdent};

/// [`diagnostic::Scope`] of errors for `#[graphql_scalar]` macro.
const ERR: diagnostic::Scope = diagnostic::Scope::ScalarAttr;

/// Expands `#[graphql_scalar]` macro into generated code.
pub(crate) fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemType>(body.clone()) {
        let attrs = parse::attr::unite(("graphql_scalar", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip(["graphql_scalar", "graphql"], ast.attrs);
        return expand_on_type_alias(attrs, ast);
    } else if let Ok(mut ast) = syn::parse2::<syn::DeriveInput>(body) {
        let attrs = parse::attr::unite(("graphql_scalar", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip(["graphql_scalar", "graphql"], ast.attrs);
        return expand_on_derive_input(attrs, ast);
    }

    Err(syn::Error::new(
        Span::call_site(),
        "#[graphql_scalar] attribute is applicable to type aliases, structs, \
         enums and unions only",
    ))
}

/// Expands `#[graphql_scalar]` macro placed on a type alias.
fn expand_on_type_alias(
    attrs: Vec<syn::Attribute>,
    ast: syn::ItemType,
) -> syn::Result<TokenStream> {
    let attr = Attr::from_attrs(["graphql_scalar", "graphql"], &attrs)?;
    if attr.transparent {
        return Err(ERR.custom_error(
            ast.span(),
            "`transparent` attribute argument isn't applicable to type aliases",
        ));
    }

    let methods = parse_type_alias_methods(&ast, &attr)?;
    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    let def = Definition {
        ty: TypeOrIdent::Type(ast.ty.clone()),
        where_clause: attr
            .where_clause
            .map_or_else(Vec::new, |cl| cl.into_inner()),
        generics: ast.generics.clone(),
        methods,
        name: attr
            .name
            .map(SpanContainer::into_inner)
            .unwrap_or_else(|| ast.ident.to_string()),
        description: attr.description.map(SpanContainer::into_inner),
        specified_by_url: attr.specified_by_url.map(SpanContainer::into_inner),
        scalar,
    };

    Ok(quote! {
        #ast
        #def
    })
}

/// Expands `#[graphql_scalar]` macro placed on a struct, enum or union.
fn expand_on_derive_input(
    attrs: Vec<syn::Attribute>,
    ast: syn::DeriveInput,
) -> syn::Result<TokenStream> {
    let attr = Attr::from_attrs(["graphql_scalar", "graphql"], &attrs)?;
    let methods = parse_derived_methods(&ast, &attr)?;
    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    let def = Definition {
        ty: TypeOrIdent::Ident(ast.ident.clone()),
        where_clause: attr
            .where_clause
            .map_or_else(Vec::new, |cl| cl.into_inner()),
        generics: ast.generics.clone(),
        methods,
        name: attr
            .name
            .map(SpanContainer::into_inner)
            .unwrap_or_else(|| ast.ident.to_string()),
        description: attr.description.map(SpanContainer::into_inner),
        specified_by_url: attr.specified_by_url.map(SpanContainer::into_inner),
        scalar,
    };

    Ok(quote! {
        #ast
        #def
    })
}

/// Parses [`Methods`] from the provided [`Attr`] for the specified type alias.
fn parse_type_alias_methods(ast: &syn::ItemType, attr: &Attr) -> syn::Result<Methods> {
    match (
        attr.to_output.as_deref().cloned(),
        attr.from_input.as_deref().cloned(),
        attr.parse_token.as_deref().cloned(),
        attr.with.as_deref().cloned(),
    ) {
        (Some(to_output), Some(from_input), Some(parse_token), None) => Ok(Methods::Custom {
            to_output,
            from_input,
            parse_token,
        }),
        (to_output, from_input, parse_token, Some(module)) => Ok(Methods::Custom {
            to_output: to_output.unwrap_or_else(|| parse_quote! { #module::to_output }),
            from_input: from_input.unwrap_or_else(|| parse_quote! { #module::from_input }),
            parse_token: parse_token
                .unwrap_or_else(|| ParseToken::Custom(parse_quote! { #module::parse_token })),
        }),
        _ => Err(ERR.custom_error(
            ast.span(),
            "all the resolvers have to be provided via `with` attribute \
             argument or a combination of `to_output_with`, `from_input_with`, \
             `parse_token_with`/`parse_token` attribute arguments",
        )),
    }
}
