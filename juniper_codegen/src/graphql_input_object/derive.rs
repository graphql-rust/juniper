//! Code generation for `#[derive(GraphQLInputObject)]` macro.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::ToTokens as _;
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned};

use crate::common::{diagnostic, rename, scalar, SpanContainer};

use super::{ContainerAttr, Definition, FieldAttr, FieldDefinition};

/// [`diagnostic::Scope`] of errors for `#[derive(GraphQLInputObject)]` macro.
const ERR: diagnostic::Scope = diagnostic::Scope::InputObjectDerive;

/// Expands `#[derive(GraphQLInputObject)]` macro into generated code.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input)?;
    let attr = ContainerAttr::from_attrs("graphql", &ast.attrs)?;

    let data = if let syn::Data::Struct(data) = &ast.data {
        data
    } else {
        return Err(ERR.custom_error(ast.span(), "can only be derived on structs"));
    };

    let renaming = attr
        .rename_fields
        .map(SpanContainer::into_inner)
        .unwrap_or(rename::Policy::CamelCase);

    let is_internal = attr.is_internal;
    let fields = data
        .fields
        .iter()
        .filter_map(|f| parse_field(f, renaming, is_internal))
        .collect::<Vec<_>>();

    diagnostic::abort_if_dirty();

    if !fields.iter().any(|f| !f.ignored) {
        return Err(ERR.custom_error(data.fields.span(), "expected at least 1 non-ignored field"));
    }

    let unique_fields = fields.iter().map(|v| &v.name).collect::<HashSet<_>>();
    if unique_fields.len() != fields.len() {
        return Err(ERR.custom_error(
            data.fields.span(),
            "expected all fields to have unique names",
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
        fields,
    };

    Ok(definition.into_token_stream())
}

/// Parses a [`FieldDefinition`] from the given struct field definition.
///
/// Returns [`None`] if the parsing fails.
fn parse_field(
    f: &syn::Field,
    renaming: rename::Policy,
    is_internal: bool,
) -> Option<FieldDefinition> {
    let field_attr = FieldAttr::from_attrs("graphql", &f.attrs)
        .map_err(diagnostic::emit_error)
        .ok()?;

    let ident = f.ident.as_ref().or_else(|| err_unnamed_field(f))?;

    let name = field_attr
        .name
        .map_or_else(
            || renaming.apply(&ident.unraw().to_string()),
            SpanContainer::into_inner,
        )
        .into_boxed_str();
    if !is_internal && name.starts_with("__") {
        ERR.no_double_underscore(f.span());
    }

    Some(FieldDefinition {
        ident: ident.clone(),
        ty: f.ty.clone(),
        default: field_attr.default.map(SpanContainer::into_inner),
        name,
        description: field_attr.description.map(SpanContainer::into_inner),
        ignored: field_attr.ignore.is_some(),
    })
}

/// Emits "expected named struct field" [`syn::Error`] pointing to the given
/// `span`.
pub(crate) fn err_unnamed_field<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.emit_custom(span.span(), "expected named struct field");
    None
}
