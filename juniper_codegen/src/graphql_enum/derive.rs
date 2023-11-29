//! Code generation for `#[derive(GraphQLEnum)]` macro.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::ToTokens as _;
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned};

use crate::common::{diagnostic, rename, scalar, SpanContainer};

use super::{ContainerAttr, Definition, ValueDefinition, VariantAttr};

/// [`diagnostic::Scope`] of errors for `#[derive(GraphQLEnum)]` macro.
const ERR: diagnostic::Scope = diagnostic::Scope::EnumDerive;

/// Expands `#[derive(GraphQLEnum)]` macro into generated code.
pub(crate) fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input)?;
    let attr = ContainerAttr::from_attrs("graphql", &ast.attrs)?;

    let data = if let syn::Data::Enum(data) = &ast.data {
        data
    } else {
        return Err(ERR.custom_error(ast.span(), "can only be derived on enums"));
    };

    let mut has_ignored_variants = false;
    let renaming = attr
        .rename_values
        .map(SpanContainer::into_inner)
        .unwrap_or(rename::Policy::ScreamingSnakeCase);
    let values = data
        .variants
        .iter()
        .filter_map(|v| {
            parse_value(v, renaming).or_else(|| {
                has_ignored_variants = true;
                None
            })
        })
        .collect::<Vec<_>>();

    diagnostic::abort_if_dirty();

    if values.is_empty() {
        return Err(ERR.custom_error(
            data.variants.span(),
            "expected at least 1 non-ignored enum variant",
        ));
    }

    let unique_values = values.iter().map(|v| &v.name).collect::<HashSet<_>>();
    if unique_values.len() != values.len() {
        return Err(ERR.custom_error(
            data.variants.span(),
            "expected all GraphQL enum values to have unique names",
        ));
    }

    let name = attr
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| ast.ident.unraw().to_string())
        .into_boxed_str();
    if !attr.is_internal && name.starts_with("__") {
        ERR.no_double_underscore(
            attr.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| ast.ident.span()),
        );
    }

    let context = attr
        .context
        .map_or_else(|| parse_quote! { () }, SpanContainer::into_inner);

    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    diagnostic::abort_if_dirty();

    let definition = Definition {
        ident: ast.ident,
        generics: ast.generics,
        name,
        description: attr.description.map(SpanContainer::into_inner),
        context,
        scalar,
        values,
        has_ignored_variants,
    };

    Ok(definition.into_token_stream())
}

/// Parses a [`ValueDefinition`] from the given Rust enum variant definition.
///
/// Returns [`None`] if the parsing fails, or the enum variant is ignored.
fn parse_value(v: &syn::Variant, renaming: rename::Policy) -> Option<ValueDefinition> {
    let attr = VariantAttr::from_attrs("graphql", &v.attrs)
        .map_err(diagnostic::emit_error)
        .ok()?;

    if attr.ignore.is_some() {
        return None;
    }

    if !v.fields.is_empty() {
        err_variant_with_fields(&v.fields)?;
    }

    let name = attr
        .name
        .map_or_else(
            || renaming.apply(&v.ident.unraw().to_string()),
            SpanContainer::into_inner,
        )
        .into_boxed_str();

    Some(ValueDefinition {
        ident: v.ident.clone(),
        name,
        description: attr.description.map(SpanContainer::into_inner),
        deprecated: attr.deprecated.map(SpanContainer::into_inner),
    })
}

/// Emits "no fields allowed for non-ignored variants" [`syn::Error`] pointing
/// to the given `span`.
pub fn err_variant_with_fields<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.emit_custom(span.span(), "no fields allowed for non-ignored variants");
    None
}
