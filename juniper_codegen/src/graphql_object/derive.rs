//! Code generation for `#[derive(GraphQLObject)]` macro.

use std::mem;

use proc_macro2::TokenStream;
use proc_macro_error::ResultExt as _;
use quote::ToTokens;
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned as _};

use crate::{
    common::{
        field,
        parse::{self, TypeExt as _},
        ScalarValueType,
    },
    result::GraphQLScope,
    util::{span_container::SpanContainer, RenameRule},
};

use super::{Attr, Definition};

/// [`GraphQLScope`] of errors for `#[derive(GraphQLObject)]` macro.
const ERR: GraphQLScope = GraphQLScope::ObjectDerive;

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
fn expand_struct(ast: syn::DeriveInput) -> syn::Result<Definition> {
    let attr = Attr::from_attrs("graphql", &ast.attrs)?;

    let struct_span = ast.span();
    let struct_ident = ast.ident;

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

    let scalar = ScalarValueType::parse(attr.scalar.as_deref(), &ast.generics);

    proc_macro_error::abort_if_dirty();

    let renaming = attr
        .rename_fields
        .as_deref()
        .copied()
        .unwrap_or(RenameRule::CamelCase);

    let mut fields = vec![];
    if let syn::Data::Struct(data) = &ast.data {
        if let syn::Fields::Named(fs) = &data.fields {
            fields = fs
                .iter()
                .filter_map(|f| parse_field(f, &renaming))
                .collect();
        } else {
            ERR.custom(struct_span, "only named fields are allowed")
                .emit();
        }
    }

    proc_macro_error::abort_if_dirty();

    if fields.is_empty() {
        ERR.emit_custom(struct_span, "must have at least one field");
    }
    if !field::all_different(&fields) {
        ERR.emit_custom(struct_span, "must have a different name for each field");
    }

    proc_macro_error::abort_if_dirty();

    Ok(Definition {
        name,
        ty: parse_quote! { #struct_ident },
        generics: ast.generics,
        description: attr.description.map(SpanContainer::into_inner),
        context: attr.context.map(SpanContainer::into_inner),
        scalar,
        fields,
        interfaces: attr
            .interfaces
            .iter()
            .map(|ty| ty.as_deref().clone())
            .collect(),
    })
}

/// Parses a [`field::Definition`] from the given Rust struct [`syn::Field`].
///
/// Returns [`None`] if parsing fails, or the struct field is ignored.
#[must_use]
fn parse_field(field: &syn::Field, renaming: &RenameRule) -> Option<field::Definition> {
    let attr = field::Attr::from_attrs("graphql", &field.attrs)
        .map_err(|e| proc_macro_error::emit_error!(e))
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

    let description = attr.description.as_ref().map(|d| d.as_ref().value());
    let deprecated = attr
        .deprecated
        .as_ref()
        .map(|d| d.as_deref().map(syn::LitStr::value));

    Some(field::Definition {
        name,
        ty,
        description,
        deprecated,
        ident: field_ident.clone(),
        arguments: None,
        has_receiver: false,
        is_async: false,
    })
}
