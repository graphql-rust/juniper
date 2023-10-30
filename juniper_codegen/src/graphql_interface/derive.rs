//! Code generation for `#[derive(GraphQLInterface)]` macro.

use proc_macro2::TokenStream;
use quote::ToTokens as _;
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned};

use crate::common::{diagnostic, field, parse::TypeExt as _, rename, scalar, SpanContainer};

use super::{attr::err_unnamed_field, enum_idents, Attr, Definition};

/// [`diagnostic::Scope`] of errors for `#[derive(GraphQLInterface)]` macro.
const ERR: diagnostic::Scope = diagnostic::Scope::InterfaceDerive;

/// Expands `#[derive(GraphQLInterface)]` macro into generated code.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input)?;
    let attr = Attr::from_attrs("graphql", &ast.attrs)?;

    let data = if let syn::Data::Struct(data) = &ast.data {
        data
    } else {
        return Err(ERR.custom_error(ast.span(), "can only be derived on structs"));
    };

    let struct_ident = &ast.ident;
    let struct_span = ast.span();

    let name = attr
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| struct_ident.unraw().to_string())
        .into_boxed_str();
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

    let fields = data
        .fields
        .iter()
        .filter_map(|f| parse_field(f, &renaming))
        .collect::<Vec<_>>();

    diagnostic::abort_if_dirty();

    if fields.is_empty() {
        ERR.emit_custom(struct_span, "must have at least one field");
    }
    if !field::all_different(&fields) {
        ERR.emit_custom(struct_span, "must have a different name for each field");
    }

    diagnostic::abort_if_dirty();

    let context = attr
        .context
        .as_deref()
        .cloned()
        .or_else(|| {
            fields.iter().find_map(|f| {
                f.arguments.as_ref().and_then(|f| {
                    f.iter()
                        .find_map(field::MethodArgument::context_ty)
                        .cloned()
                })
            })
        })
        .unwrap_or_else(|| parse_quote! { () });

    let (enum_ident, enum_alias_ident) = enum_idents(struct_ident, attr.r#enum.as_deref());

    Ok(Definition {
        generics: ast.generics.clone(),
        vis: ast.vis.clone(),
        enum_ident,
        enum_alias_ident,
        name,
        description: attr.description.map(SpanContainer::into_inner),
        context,
        scalar,
        fields,
        implemented_for: attr
            .implemented_for
            .into_iter()
            .map(SpanContainer::into_inner)
            .collect(),
        implements: attr
            .implements
            .into_iter()
            .map(SpanContainer::into_inner)
            .collect(),
        suppress_dead_code: Some((ast.ident.clone(), data.fields.clone())),
        src_intra_doc_link: format!("struct@{struct_ident}").into_boxed_str(),
    }
    .into_token_stream())
}

/// Parses a [`field::Definition`] from the given struct field definition.
///
/// Returns [`None`] if the parsing fails, or the struct field is ignored.
#[must_use]
fn parse_field(field: &syn::Field, renaming: &rename::Policy) -> Option<field::Definition> {
    let field_ident = field.ident.as_ref().or_else(|| err_unnamed_field(&field))?;

    let attr = field::Attr::from_attrs("graphql", &field.attrs)
        .map_err(diagnostic::emit_error)
        .ok()?;

    if attr.ignore.is_some() {
        return None;
    }

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

    let mut ty = field.ty.clone();
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
