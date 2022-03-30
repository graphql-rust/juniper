//! Code generation for `#[derive(GraphQLEnum)]` macro.

use proc_macro2::TokenStream;
use quote::ToTokens as _;
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned};

use crate::{
    common::scalar,
    result::GraphQLScope,
    util::{span_container::SpanContainer, RenameRule},
};

use super::{ContainerAttr, Definition, VariantAttr, VariantDefinition};

/// [`GraphQLScope`] of errors for `#[derive(GraphQLEnum)]` macro.
const ERR: GraphQLScope = GraphQLScope::EnumDerive;

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input)?;
    let attr = ContainerAttr::from_attrs("graphql", &ast.attrs)?;

    let data = if let syn::Data::Enum(data) = &ast.data {
        data
    } else {
        return Err(ERR.custom_error(ast.span(), "can only be derived on enums"));
    };

    let mut has_ignored_variants = false;
    let renaming = attr
        .rename
        .map(SpanContainer::into_inner)
        .unwrap_or(RenameRule::ScreamingSnakeCase);
    let variants = data
        .variants
        .iter()
        .filter_map(|v| {
            parse_variant(v, renaming).or_else(|| {
                has_ignored_variants = true;
                None
            })
        })
        .collect::<Vec<_>>();

    proc_macro_error::abort_if_dirty();

    if variants.is_empty() {
        return Err(ERR.custom_error(
            data.variants.span(),
            "expected at least 1 non-ignored variant",
        ));
    }

    let name = attr
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| ast.ident.unraw().to_string());
    if !attr.is_internal && name.starts_with("__") {
        ERR.no_double_underscore(
            attr.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| ast.ident.span()),
        );
    }

    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    proc_macro_error::abort_if_dirty();

    let definition = Definition {
        ident: ast.ident.clone(),
        generics: ast.generics,
        name,
        description: attr.description.map(SpanContainer::into_inner),
        context: attr
            .context
            .map_or_else(|| parse_quote! { () }, SpanContainer::into_inner),
        scalar,
        variants,
        has_ignored_variants,
    };

    Ok(definition.into_token_stream())
}

/// Parses a [`VariantDefinition`] from the given struct field definition.
///
/// Returns [`None`] if the parsing fails, or the enum variant is ignored.
fn parse_variant(v: &syn::Variant, renaming: RenameRule) -> Option<VariantDefinition> {
    let var_attr = VariantAttr::from_attrs("graphql", &v.attrs)
        .map_err(|e| proc_macro_error::emit_error!(e))
        .ok()?;

    if var_attr.ignore.is_some() {
        return None;
    }

    if !v.fields.is_empty() {
        err_variant_with_fields(&v.fields)?;
    }

    let name = var_attr.name.map_or_else(
        || renaming.apply(&v.ident.unraw().to_string()),
        |name| name.into_inner(),
    );

    let description = var_attr.description.map(SpanContainer::into_inner);

    let deprecated = var_attr
        .deprecated
        .map(|desc| desc.into_inner().as_ref().map(syn::LitStr::value));

    Some(VariantDefinition {
        ident: v.ident.clone(),
        name,
        description,
        deprecated,
    })
}

/// Emits "no fields allows for non-ignored variants" [`syn::Error`] pointing to
/// the given `span`.
pub fn err_variant_with_fields<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.emit_custom(span.span(), "no fields allows for non-ignored variants");
    None
}
