//! Code generation for `#[derive(GraphQLObject)]` macro.

use std::marker::PhantomData;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned as _};

use crate::common::{
    diagnostic::{self, ResultExt as _},
    field,
    parse::TypeExt as _,
    rename, scalar, SpanContainer,
};

use super::{Attr, Definition, Query};

/// [`diagnostic::Scope`] of errors for `#[derive(GraphQLObject)]` macro.
const ERR: diagnostic::Scope = diagnostic::Scope::ObjectDerive;

/// Expands `#[derive(GraphQLObject)]` macro into generated code.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input).unwrap_or_abort();

    match &ast.data {
        syn::Data::Struct(_) => expand_struct(ast),
        _ => Err(ERR.custom_error(ast.span(), "can only be derived for structs")),
    }
    .map(ToTokens::into_token_stream)
}

/// Expands into generated code a `#[derive(GraphQLObject)]` macro placed on a
/// Rust struct.
fn expand_struct(ast: syn::DeriveInput) -> syn::Result<Definition<Query>> {
    let attr = Attr::from_attrs("graphql", &ast.attrs)?;

    let struct_span = ast.span();
    let struct_ident = ast.ident;

    let (_, struct_generics, _) = ast.generics.split_for_impl();
    let ty = parse_quote! { #struct_ident #struct_generics };

    let name = attr
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| struct_ident.unraw().to_string());
    if !attr.is_internal && name.starts_with("__") {
        ERR.no_double_underscore(
            attr.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| struct_ident.span()),
        );
    }

    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    diagnostic::abort_if_dirty();

    let renaming = attr
        .rename_fields
        .as_deref()
        .copied()
        .unwrap_or(rename::Policy::CamelCase);

    let mut fields = vec![];
    if let syn::Data::Struct(data) = &ast.data {
        if let syn::Fields::Named(fs) = &data.fields {
            fields = fs
                .named
                .iter()
                .filter_map(|f| parse_field(f, &renaming))
                .collect();
        } else {
            ERR.emit_custom(struct_span, "only named fields are allowed");
        }
    }

    diagnostic::abort_if_dirty();

    if fields.is_empty() {
        ERR.emit_custom(struct_span, "must have at least one field");
    }
    if !field::all_different(&fields) {
        ERR.emit_custom(struct_span, "must have a different name for each field");
    }

    diagnostic::abort_if_dirty();

    Ok(Definition {
        name,
        ty,
        generics: ast.generics,
        description: attr.description.map(SpanContainer::into_inner),
        context: attr
            .context
            .map(SpanContainer::into_inner)
            .unwrap_or_else(|| parse_quote! { () }),
        scalar,
        fields,
        interfaces: attr
            .interfaces
            .iter()
            .map(|ty| ty.as_ref().clone())
            .collect(),
        _operation: PhantomData,
    })
}

/// Parses a [`field::Definition`] from the given Rust struct [`syn::Field`].
///
/// Returns [`None`] if parsing fails, or the struct field is ignored.
#[must_use]
fn parse_field(field: &syn::Field, renaming: &rename::Policy) -> Option<field::Definition> {
    let attr = field::Attr::from_attrs("graphql", &field.attrs)
        .map_err(diagnostic::emit_error)
        .ok()?;

    if attr.ignore.is_some() {
        return None;
    }

    let field_ident = field.ident.as_ref().unwrap();

    let name = attr
        .name
        .as_ref()
        .map(|m| m.as_ref().value())
        .unwrap_or_else(|| renaming.apply(&field_ident.unraw().to_string()));
    if name.starts_with("__") {
        ERR.no_double_underscore(
            attr.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| field_ident.span()),
        );
        return None;
    }

    let mut ty = field.ty.unparenthesized().clone();
    ty.lifetimes_anonymized();

    Some(field::Definition {
        name,
        ty,
        description: attr.description.map(SpanContainer::into_inner),
        deprecated: attr.deprecated.map(SpanContainer::into_inner),
        ident: field_ident.clone(),
        arguments: None,
        has_receiver: false,
        is_async: false,
    })
}
