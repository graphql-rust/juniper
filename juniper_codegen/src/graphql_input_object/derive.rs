//! Code generation for `#[derive(GraphQLInputObject)]` macro.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::ToTokens as _;
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned};

use crate::common::{SpanContainer, diagnostic, rename, scalar};

use super::{ContainerAttr, Definition, FieldAttr, FieldDefinition};

/// [`diagnostic::Scope`] of errors for `#[derive(GraphQLInputObject)]` macro.
const ERR: diagnostic::Scope = diagnostic::Scope::InputObjectDerive;

/// Expands `#[derive(GraphQLInputObject)]` macro placed on a struct or an enum.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input)?;
    let attr = ContainerAttr::from_attrs("graphql", &ast.attrs)?;

    let renaming = attr
        .rename_fields
        .map(SpanContainer::into_inner)
        .unwrap_or(rename::Policy::CamelCase);
    let is_internal = attr.is_internal;
    let (fields, fields_span) = match &ast.data {
        syn::Data::Struct(data) => {
            let fields = data
                .fields
                .iter()
                .filter_map(|f| parse_struct_field(f, renaming, is_internal))
                .collect::<Vec<_>>();
            (fields, data.fields.span())
        }
        syn::Data::Enum(data) => {
            let fields = data
                .variants
                .iter()
                .filter_map(|v| parse_enum_variant(v, renaming, is_internal))
                .collect::<Vec<_>>();
            (fields, data.variants.span())
        }
        syn::Data::Union(_) => {
            return Err(ERR.custom_error(ast.span(), "cannot be derived on unions"));
        }
    };

    diagnostic::abort_if_dirty();

    if !fields.iter().any(|f| !f.ignored) {
        return Err(ERR.custom_error(fields_span, "expected at least 1 non-ignored field"));
    }

    let unique_fields = fields.iter().map(|v| &v.name).collect::<HashSet<_>>();
    if unique_fields.len() != fields.len() {
        return Err(ERR.custom_error(fields_span, "expected all fields to have unique names"));
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
        is_one_of: matches!(ast.data, syn::Data::Enum(_)),
    };

    Ok(definition.into_token_stream())
}

/// Parses a [`FieldDefinition`] from the provided struct field definition.
///
/// Returns [`None`] if the parsing fails.
fn parse_struct_field(
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
        deprecated: field_attr.deprecated.map(SpanContainer::into_inner),
        ignored: field_attr.ignore.is_some(),
    })
}

/// Parses a [`FieldDefinition`] from the provided enum variant definition.
///
/// Returns [`None`] if the parsing fails.
fn parse_enum_variant(
    v: &syn::Variant,
    renaming: rename::Policy,
    is_internal: bool,
) -> Option<FieldDefinition> {
    if v.fields.len() != 1 || !matches!(v.fields, syn::Fields::Unnamed(_)) {
        ERR.emit_custom(
            v.fields.span(),
            "enum variant must have exactly 1 unnamed field to represent `@oneOf` input object \
             field",
        );
    }

    let field_attr = FieldAttr::from_attrs("graphql", &v.attrs)
        .map_err(diagnostic::emit_error)
        .ok()?;

    let ignored = field_attr.ignore.is_some();
    if let Some(default) = &field_attr.default {
        ERR.emit_custom(
            default.span_ident(),
            if ignored {
                "`default` attribute argument has no meaning for ignored variants, as they are \
                 never constructed"
            } else {
                "field cannot have default value in `@oneOf` input object"
            },
        );
    }

    let ident = &v.ident;

    let name = field_attr
        .name
        .map_or_else(
            || {
                let mut name = ident.unraw().to_string();
                if renaming != rename::Policy::None {
                    // Make naming similar to struct fields before applying further renaming.
                    name = rename::Policy::SnakeCase.apply(&ident.unraw().to_string());
                }
                renaming.apply(&name)
            },
            SpanContainer::into_inner,
        )
        .into_boxed_str();
    if !is_internal && name.starts_with("__") {
        ERR.no_double_underscore(v.span());
    }

    let field_ty = v.fields.iter().next().unwrap().ty.clone();

    Some(FieldDefinition {
        ident: ident.clone(),
        ty: parse_quote! { ::core::option::Option<#field_ty> },
        default: None,
        name,
        description: field_attr.description.map(SpanContainer::into_inner),
        deprecated: field_attr.deprecated.map(SpanContainer::into_inner),
        ignored,
    })
}

/// Emits "expected named struct field" [`syn::Error`] pointing to the provided `span`.
pub(crate) fn err_unnamed_field<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.emit_custom(span.span(), "expected named struct field");
    None
}
