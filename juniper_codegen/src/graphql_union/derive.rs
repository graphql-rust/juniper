//! Code generation for `#[derive(GraphQLUnion)]` macro.

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned as _, Data, Fields};

use crate::common::{
    diagnostic::{self, ResultExt as _},
    parse::TypeExt as _,
    scalar, SpanContainer,
};

use super::{
    all_variants_different, emerge_union_variants_from_attr, Attr, Definition, VariantAttr,
    VariantDefinition,
};

/// [`diagnostic::Scope`] of errors for `#[derive(GraphQLUnion)]` macro.
const ERR: diagnostic::Scope = diagnostic::Scope::UnionDerive;

/// Expands `#[derive(GraphQLUnion)]` macro into generated code.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input).unwrap_or_abort();

    match &ast.data {
        Data::Enum(_) => expand_enum(ast),
        Data::Struct(_) => expand_struct(ast),
        _ => Err(ERR.custom_error(ast.span(), "can only be derived for enums and structs")),
    }
    .map(ToTokens::into_token_stream)
}

/// Expands into generated code a `#[derive(GraphQLUnion)]` macro placed on a
/// Rust enum.
fn expand_enum(ast: syn::DeriveInput) -> syn::Result<Definition> {
    let attr = Attr::from_attrs("graphql", &ast.attrs)?;

    let enum_span = ast.span();
    let enum_ident = ast.ident;

    let name = attr
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| enum_ident.unraw().to_string());
    if !attr.is_internal && name.starts_with("__") {
        ERR.no_double_underscore(
            attr.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| enum_ident.span()),
        );
    }

    let mut variants: Vec<_> = match ast.data {
        Data::Enum(data) => data.variants,
        data => unreachable!("graphql_union::derive::expand_enum({data:?})"),
    }
    .into_iter()
    .filter_map(|var| parse_variant_from_enum_variant(var, &enum_ident, &attr))
    .collect();

    diagnostic::abort_if_dirty();

    emerge_union_variants_from_attr(&mut variants, attr.external_resolvers);

    if variants.is_empty() {
        ERR.emit_custom(enum_span, "expects at least one union variant");
    }

    if !all_variants_different(&variants) {
        ERR.emit_custom(
            enum_span,
            "must have a different type for each union variant",
        );
    }

    diagnostic::abort_if_dirty();

    Ok(Definition {
        name,
        ty: parse_quote! { #enum_ident },
        is_trait_object: false,
        description: attr.description.map(SpanContainer::into_inner),
        context: attr
            .context
            .map(SpanContainer::into_inner)
            .unwrap_or_else(|| parse_quote! { () }),
        scalar: scalar::Type::parse(attr.scalar.as_deref(), &ast.generics),
        generics: ast.generics,
        variants,
    })
}

/// Parses given Rust enum `var`iant as [GraphQL union][1] variant.
///
/// On failure returns [`None`] and internally fills up [`diagnostic`]
/// with the corresponding errors.
///
/// [1]: https://spec.graphql.org/October2021#sec-Unions
fn parse_variant_from_enum_variant(
    var: syn::Variant,
    enum_ident: &syn::Ident,
    enum_attr: &Attr,
) -> Option<VariantDefinition> {
    let attr = VariantAttr::from_attrs("graphql", &var.attrs)
        .map_err(diagnostic::emit_error)
        .ok()?;
    if attr.ignore.is_some() {
        return None;
    }

    let var_ident = var.ident;

    let ty = match var.fields {
        Fields::Unnamed(fields) => {
            let mut iter = fields.unnamed.iter();
            let first = iter.next().unwrap();
            if iter.next().is_none() {
                Ok(first.ty.unparenthesized().clone())
            } else {
                Err(fields.span())
            }
        }
        _ => Err(var_ident.span()),
    }
    .map_err(|span| {
        ERR.emit_custom(
            span,
            "enum allows only unnamed variants with a single field, e.g. `Some(T)`",
        )
    })
    .ok()?;

    let enum_path = quote! { #enum_ident::#var_ident };

    let resolver_code = if let Some(rslvr) = attr.external_resolver {
        if let Some(other) = enum_attr.external_resolvers.get(&ty) {
            ERR.emit_custom(
                rslvr.span_ident(),
                format!(
                    "variant `{}` already has external resolver function `{}` \
                     declared on the enum",
                    ty.to_token_stream(),
                    other.to_token_stream(),
                ),
            );
        }

        let resolver_fn = rslvr.into_inner();

        parse_quote! {
            #resolver_fn(self, ::juniper::FromContext::from(context))
        }
    } else {
        parse_quote! {
            match self {
                #enum_ident::#var_ident(ref v) => ::core::option::Option::Some(v),
                _ => ::core::option::Option::None,
            }
        }
    };

    let resolver_check = parse_quote! {
        ::core::matches!(self, #enum_path(_))
    };

    Some(VariantDefinition {
        ty,
        resolver_code,
        resolver_check,
        context: None,
    })
}

/// Expands into generated code a `#[derive(GraphQLUnion)]` macro placed on a
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

    let mut variants = vec![];
    emerge_union_variants_from_attr(&mut variants, attr.external_resolvers);
    if variants.is_empty() {
        ERR.emit_custom(struct_span, "expects at least one union variant");
    }

    if !all_variants_different(&variants) {
        ERR.emit_custom(
            struct_span,
            "must have a different type for each union variant",
        );
    }

    diagnostic::abort_if_dirty();

    Ok(Definition {
        name,
        ty: parse_quote! { #struct_ident },
        is_trait_object: false,
        description: attr.description.map(SpanContainer::into_inner),
        context: attr
            .context
            .map(SpanContainer::into_inner)
            .unwrap_or_else(|| parse_quote! { () }),
        scalar: scalar::Type::parse(attr.scalar.as_deref(), &ast.generics),
        generics: ast.generics,
        variants,
    })
}
