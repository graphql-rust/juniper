//! Code generation for `#[derive(GraphQLUnion)]`/`#[derive(GraphQLUnionInternal)]` macros.

use proc_macro2::TokenStream;
use proc_macro_error::ResultExt as _;
use quote::{quote, ToTokens};
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned as _, Data, Fields};

use crate::{
    result::GraphQLScope,
    util::{span_container::SpanContainer, to_pascal_case, Mode},
};

use super::{
    all_variants_different, emerge_union_variants_from_meta, UnionDefinition, UnionMeta,
    UnionVariantDefinition, UnionVariantMeta,
};

/// [`GraphQLScope`] of `#[derive(GraphQLUnion)]`/`#[derive(GraphQLUnionInternal)]` macros.
const SCOPE: GraphQLScope = GraphQLScope::UnionDerive;

/// Expands `#[derive(GraphQLUnion)]`/`#[derive(GraphQLUnionInternal)]` macro into generated code.
pub fn expand(input: TokenStream, mode: Mode) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input).unwrap_or_abort();

    match &ast.data {
        Data::Enum(_) => expand_enum(ast, mode),
        Data::Struct(_) => expand_struct(ast, mode),
        _ => Err(SCOPE.custom_error(ast.span(), "can only be applied to enums and structs")),
    }
    .map(ToTokens::into_token_stream)
}

/// Expands into generated code a `#[derive(GraphQLUnion)]`/`#[derive(GraphQLUnionInternal)]` macro
/// placed on a Rust enum.
fn expand_enum(ast: syn::DeriveInput, mode: Mode) -> syn::Result<UnionDefinition> {
    let meta = UnionMeta::from_attrs("graphql", &ast.attrs)?;

    let enum_span = ast.span();
    let enum_ident = ast.ident;

    let name = meta
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| to_pascal_case(&enum_ident.unraw().to_string()));
    if matches!(mode, Mode::Public) && name.starts_with("__") {
        SCOPE.no_double_underscore(
            meta.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| enum_ident.span()),
        );
    }

    let mut variants: Vec<_> = match ast.data {
        Data::Enum(data) => data.variants,
        _ => unreachable!(),
    }
    .into_iter()
    .filter_map(|var| parse_variant_from_enum_variant(var, &enum_ident, &meta, mode))
    .collect();

    proc_macro_error::abort_if_dirty();

    emerge_union_variants_from_meta(&mut variants, meta.custom_resolvers, mode);

    if variants.is_empty() {
        SCOPE.custom(enum_span, "expects at least one union variant");
    }

    if !all_variants_different(&variants) {
        SCOPE.custom(enum_span, "each union variant must have a different type");
    }

    proc_macro_error::abort_if_dirty();

    Ok(UnionDefinition {
        name,
        ty: parse_quote! { #enum_ident },
        is_trait_object: false,
        description: meta.description.map(SpanContainer::into_inner),
        context: meta.context.map(SpanContainer::into_inner),
        scalar: meta.scalar.map(SpanContainer::into_inner),
        generics: ast.generics,
        variants,
        span: enum_span,
        mode,
    })
}

/// Parses given Rust enum `var`iant as [GraphQL union][1] variant.
///
/// On failure returns [`None`] and internally fills up [`proc_macro_error`] with the corresponding
/// errors.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
fn parse_variant_from_enum_variant(
    var: syn::Variant,
    enum_ident: &syn::Ident,
    enum_meta: &UnionMeta,
    mode: Mode,
) -> Option<UnionVariantDefinition> {
    let meta = UnionVariantMeta::from_attrs("graphql", &var.attrs)
        .map_err(|e| proc_macro_error::emit_error!(e))
        .ok()?;
    if meta.ignore.is_some() {
        return None;
    }

    let var_span = var.span();
    let var_ident = var.ident;

    let ty = match var.fields {
        Fields::Unnamed(fields) => {
            let mut iter = fields.unnamed.iter();
            let first = iter.next().unwrap();
            if iter.next().is_none() {
                Ok(first.ty.clone())
            } else {
                Err(fields.span())
            }
        }
        _ => Err(var_ident.span()),
    }
    .map_err(|span| {
        SCOPE.custom(
            span,
            "only unnamed variants with a single field are allowed, e.g. Some(T)",
        )
    })
    .ok()?;

    let enum_path = quote! { #enum_ident::#var_ident };

    let resolver_code = if let Some(rslvr) = meta.custom_resolver {
        if let Some(other) = enum_meta.custom_resolvers.get(&ty) {
            SCOPE.custom(
                rslvr.span_ident(),
                format!(
                    "variant `{}` already has custom resolver `{}` declared on the enum",
                    ty.to_token_stream(),
                    other.to_token_stream(),
                ),
            );
        }

        let crate_path = mode.crate_path();
        let resolver_fn = rslvr.into_inner();

        parse_quote! {
            #resolver_fn(self, #crate_path::FromContext::from(context))
        }
    } else {
        parse_quote! {
            match self { #enum_ident::#var_ident(ref v) => Some(v), _ => None, }
        }
    };

    let resolver_check = parse_quote! {
        matches!(self, #enum_path(_))
    };

    Some(UnionVariantDefinition {
        ty,
        resolver_code,
        resolver_check,
        enum_path: Some(enum_path),
        span: var_span,
    })
}

/// Expands into generated code a `#[derive(GraphQLUnion)]`/`#[derive(GraphQLUnionInternal)]` macro
/// placed on a Rust struct.
fn expand_struct(ast: syn::DeriveInput, mode: Mode) -> syn::Result<UnionDefinition> {
    let meta = UnionMeta::from_attrs("graphql", &ast.attrs)?;

    let struct_span = ast.span();
    let struct_ident = ast.ident;

    let name = meta
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| to_pascal_case(&struct_ident.unraw().to_string()));
    if matches!(mode, Mode::Public) && name.starts_with("__") {
        SCOPE.no_double_underscore(
            meta.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| struct_ident.span()),
        );
    }

    let mut variants = vec![];
    emerge_union_variants_from_meta(&mut variants, meta.custom_resolvers, mode);
    if variants.is_empty() {
        SCOPE.custom(struct_span, "expects at least one union variant");
    }

    if !all_variants_different(&variants) {
        SCOPE.custom(struct_span, "each union variant must have a different type");
    }

    proc_macro_error::abort_if_dirty();

    Ok(UnionDefinition {
        name,
        ty: parse_quote! { #struct_ident },
        is_trait_object: false,
        description: meta.description.map(SpanContainer::into_inner),
        context: meta.context.map(SpanContainer::into_inner),
        scalar: meta.scalar.map(SpanContainer::into_inner),
        generics: ast.generics,
        variants,
        span: struct_span,
        mode,
    })
}
