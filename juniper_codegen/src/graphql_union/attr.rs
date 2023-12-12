//! Code generation for `#[graphql_union]` macro.

use std::mem;

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens as _};
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned as _};

use crate::common::{diagnostic, parse, path_eq_single, scalar, SpanContainer};

use super::{
    all_variants_different, emerge_union_variants_from_attr, Attr, Definition, VariantAttr,
    VariantDefinition,
};

/// [`diagnostic::Scope`] of errors for `#[graphql_union]` macro.
const ERR: diagnostic::Scope = diagnostic::Scope::UnionAttr;

/// Expands `#[graphql_union]` macro into generated code.
pub fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemTrait>(body) {
        let trait_attrs = parse::attr::unite(("graphql_union", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip(["graphql_union", "graphql"], ast.attrs);
        return expand_on_trait(trait_attrs, ast);
    }

    Err(syn::Error::new(
        Span::call_site(),
        "#[graphql_union] attribute is applicable to trait definitions only",
    ))
}

/// Expands `#[graphql_union]` macro placed on a trait definition.
fn expand_on_trait(
    attrs: Vec<syn::Attribute>,
    mut ast: syn::ItemTrait,
) -> syn::Result<TokenStream> {
    let attr = Attr::from_attrs(["graphql_union", "graphql"], &attrs)?;

    let trait_span = ast.span();
    let trait_ident = &ast.ident;

    let name = attr
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| trait_ident.unraw().to_string());
    if !attr.is_internal && name.starts_with("__") {
        ERR.no_double_underscore(
            attr.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| trait_ident.span()),
        );
    }

    let mut variants: Vec<_> = ast
        .items
        .iter_mut()
        .filter_map(|i| match i {
            syn::TraitItem::Fn(m) => parse_variant_from_trait_method(m, trait_ident, &attr),
            _ => None,
        })
        .collect();

    diagnostic::abort_if_dirty();

    emerge_union_variants_from_attr(&mut variants, attr.external_resolvers);

    if variants.is_empty() {
        ERR.emit_custom(trait_span, "expects at least one union variant");
    }

    if !all_variants_different(&variants) {
        ERR.emit_custom(
            trait_span,
            "must have a different type for each union variant",
        );
    }

    diagnostic::abort_if_dirty();

    let context = attr
        .context
        .map(SpanContainer::into_inner)
        .or_else(|| variants.iter().find_map(|v| v.context.as_ref()).cloned())
        .unwrap_or_else(|| parse_quote! { () });

    let generated_code = Definition {
        name,
        ty: parse_quote! { #trait_ident },
        is_trait_object: true,
        description: attr.description.map(SpanContainer::into_inner),
        context,
        scalar: scalar::Type::parse(attr.scalar.as_deref(), &ast.generics),
        generics: ast.generics.clone(),
        variants,
    };

    Ok(quote! {
        #ast
        #generated_code
    })
}

/// Parses given Rust trait `method` as [GraphQL union][1] variant.
///
/// On failure returns [`None`] and internally fills up [`diagnostic`]
/// with the corresponding errors.
///
/// [1]: https://spec.graphql.org/October2021#sec-Unions
fn parse_variant_from_trait_method(
    method: &mut syn::TraitItemFn,
    trait_ident: &syn::Ident,
    trait_attr: &Attr,
) -> Option<VariantDefinition> {
    let method_attrs = method.attrs.clone();

    // Remove repeated attributes from the method, to omit incorrect expansion.
    method.attrs = mem::take(&mut method.attrs)
        .into_iter()
        .filter(|attr| !path_eq_single(attr.path(), "graphql"))
        .collect();

    let attr = VariantAttr::from_attrs("graphql", &method_attrs)
        .map_err(diagnostic::emit_error)
        .ok()?;

    if let Some(rslvr) = attr.external_resolver {
        ERR.custom(
            rslvr.span_ident(),
            "cannot use #[graphql(with = ...)] attribute on a trait method",
        )
        .note(String::from(
            "instead use #[graphql(ignore)] on the method with \
             #[graphql_union(on ... = ...)] on the trait itself",
        ))
        .emit()
    }
    if attr.ignore.is_some() {
        return None;
    }

    let method_span = method.sig.span();
    let method_ident = &method.sig.ident;

    let ty = parse::downcaster::output_type(&method.sig.output)
        .map_err(|span| {
            ERR.emit_custom(
                span,
                "expects trait method return type to be `Option<&VariantType>` only",
            )
        })
        .ok()?;
    let method_context_ty = parse::downcaster::context_ty(&method.sig)
        .map_err(|span| {
            ERR.emit_custom(
                span,
                "expects trait method to accept `&self` only and, optionally, `&Context`",
            )
        })
        .ok()?;
    if let Some(is_async) = &method.sig.asyncness {
        ERR.emit_custom(
            is_async.span(),
            "async downcast to union variants is not supported",
        );
        return None;
    }

    let resolver_code = {
        if let Some(other) = trait_attr.external_resolvers.get(&ty) {
            ERR.custom(
                method_span,
                format!(
                    "trait method `{method_ident}` conflicts with the external \
                     resolver function `{}` declared on the trait to resolve \
                     the variant type `{}`",
                    other.to_token_stream(),
                    ty.to_token_stream(),
                ),
            )
            .note(String::from(
                "use `#[graphql(ignore)]` attribute to ignore this trait \
                 method for union variants resolution",
            ))
            .emit();
        }

        if method_context_ty.is_some() {
            parse_quote! {
                #trait_ident::#method_ident(self, ::juniper::FromContext::from(context))
            }
        } else {
            parse_quote! {
                #trait_ident::#method_ident(self)
            }
        }
    };

    // Doing this may be quite an expensive, because resolving may contain some
    // heavy computation, so we're preforming it twice. Unfortunately, we have
    // no other options here, until the `juniper::GraphQLType` itself will allow
    // to do it in some cleverer way.
    let resolver_check = parse_quote! {
        ({ #resolver_code } as ::core::option::Option<&#ty>).is_some()
    };

    Some(VariantDefinition {
        ty,
        resolver_code,
        resolver_check,
        context: method_context_ty,
    })
}
