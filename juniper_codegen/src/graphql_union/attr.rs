//! Code generation for `#[graphql_union]`/`#[graphql_union_internal]` macros.

use std::{collections::HashSet, mem, ops::Deref as _};

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens as _};
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned as _};

use crate::{
    result::GraphQLScope,
    util::{path_eq_single, span_container::SpanContainer, to_pascal_case, unparenthesize, Mode},
};

use super::{
    all_variants_different, emerge_union_variants_from_meta, UnionDefinition, UnionMeta,
    UnionVariantDefinition, UnionVariantMeta,
};

/// [`GraphQLScope`] of `#[graphql_union]`/`#[graphql_union_internal]` macros.
const SCOPE: GraphQLScope = GraphQLScope::UnionAttr;

/// Returns the concrete name of the `proc_macro_attribute` for deriving `GraphQLUnion`
/// implementation depending on the provided `mode`.
fn attr_path(mode: Mode) -> &'static str {
    match mode {
        Mode::Public => "graphql_union",
        Mode::Internal => "graphql_union_internal",
    }
}

/// Expands `#[graphql_union]`/`#[graphql_union_internal]` macro into generated code.
pub fn expand(attr_args: TokenStream, body: TokenStream, mode: Mode) -> syn::Result<TokenStream> {
    let attr_path = attr_path(mode);

    let mut ast = syn::parse2::<syn::ItemTrait>(body).map_err(|_| {
        syn::Error::new(
            Span::call_site(),
            format!(
                "#[{}] attribute is applicable to trait definitions only",
                attr_path,
            ),
        )
    })?;

    let mut trait_attrs = Vec::with_capacity(ast.attrs.len() + 1);
    trait_attrs.push({
        let attr_path = syn::Ident::new(attr_path, Span::call_site());
        parse_quote! { #[#attr_path(#attr_args)] }
    });
    trait_attrs.extend_from_slice(&ast.attrs);

    // Remove repeated attributes from the definition, to omit duplicate expansion.
    ast.attrs = ast
        .attrs
        .into_iter()
        .filter_map(|attr| {
            if path_eq_single(&attr.path, attr_path) {
                None
            } else {
                Some(attr)
            }
        })
        .collect();

    let meta = UnionMeta::from_attrs(attr_path, &trait_attrs)?;

    let trait_span = ast.span();
    let trait_ident = &ast.ident;

    let name = meta
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| to_pascal_case(&trait_ident.unraw().to_string()));
    if matches!(mode, Mode::Public) && name.starts_with("__") {
        SCOPE.no_double_underscore(
            meta.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| trait_ident.span()),
        );
    }

    let mut variants: Vec<_> = ast
        .items
        .iter_mut()
        .filter_map(|i| match i {
            syn::TraitItem::Method(m) => {
                parse_variant_from_trait_method(m, trait_ident, &meta, mode)
            }
            _ => None,
        })
        .collect();

    if meta.context.is_none() && !all_contexts_different(&variants) {
        SCOPE.custom(
            trait_span,
            "cannot infer the appropriate context type, either specify the #[{}(context = MyType)] \
             attribute, so the trait methods can use any type which impls \
             `juniper::FromContext<MyType>`, or use the same type for all context arguments in \
             trait methods signatures",
        );
    }

    proc_macro_error::abort_if_dirty();

    emerge_union_variants_from_meta(&mut variants, meta.custom_resolvers, mode);

    if variants.is_empty() {
        SCOPE.custom(trait_span, "expects at least one union variant");
    }

    if !all_variants_different(&variants) {
        SCOPE.custom(
            trait_span,
            "must have a different type for each union variant",
        );
    }

    proc_macro_error::abort_if_dirty();

    let context = meta
        .context
        .map(SpanContainer::into_inner)
        .or_else(|| variants.iter().find_map(|v| v.context_ty.as_ref()).cloned());

    let generated_code = UnionDefinition {
        name,
        ty: parse_quote! { #trait_ident },
        is_trait_object: true,
        description: meta.description.map(SpanContainer::into_inner),
        context,
        scalar: meta.scalar.map(SpanContainer::into_inner),
        generics: ast.generics.clone(),
        variants,
        span: trait_span,
        mode,
    };

    Ok(quote! {
        #ast

        #generated_code
    })
}

/// Parses given Rust trait `method` as [GraphQL union][1] variant.
///
/// On failure returns [`None`] and internally fills up [`proc_macro_error`] with the corresponding
/// errors.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
fn parse_variant_from_trait_method(
    method: &mut syn::TraitItemMethod,
    trait_ident: &syn::Ident,
    trait_meta: &UnionMeta,
    mode: Mode,
) -> Option<UnionVariantDefinition> {
    let attr_path = attr_path(mode);
    let method_attrs = method.attrs.clone();

    // Remove repeated attributes from the method, to omit incorrect expansion.
    method.attrs = mem::take(&mut method.attrs)
        .into_iter()
        .filter_map(|attr| {
            if path_eq_single(&attr.path, attr_path) {
                None
            } else {
                Some(attr)
            }
        })
        .collect();

    let meta = UnionVariantMeta::from_attrs(attr_path, &method_attrs)
        .map_err(|e| proc_macro_error::emit_error!(e))
        .ok()?;

    if let Some(rslvr) = meta.custom_resolver {
        SCOPE.custom(
            rslvr.span_ident(),
            format!(
                "cannot use #[{0}(with = ...)] attribute on a trait method, instead use \
                 #[{0}(ignore)] on the method with #[{0}(on ... = ...)] on the trait itself",
                attr_path,
            ),
        )
    }
    if meta.ignore.is_some() {
        return None;
    }

    let method_span = method.sig.span();
    let method_ident = &method.sig.ident;

    let ty = parse_trait_method_output_type(&method.sig)
        .map_err(|span| {
            SCOPE.custom(
                span,
                "expects trait method return type to be `Option<&VariantType>` only",
            )
        })
        .ok()?;
    let method_context_ty = parse_trait_method_input_args(&method.sig)
        .map_err(|span| {
            SCOPE.custom(
                span,
                "expects trait method to accept `&self` only and, optionally, `&Context`",
            )
        })
        .ok()?;
    if let Some(is_async) = &method.sig.asyncness {
        SCOPE.custom(
            is_async.span(),
            "doesn't support async union variants resolvers yet",
        );
        return None;
    }

    let resolver_code = {
        if let Some(other) = trait_meta.custom_resolvers.get(&ty) {
            SCOPE.custom(
                method_span,
                format!(
                    "trait method `{}` conflicts with the custom resolver `{}` declared on the \
                     trait to resolve the variant type `{}`, use `#[{}(ignore)]` attribute to \
                     ignore this trait method for union variants resolution",
                    method_ident,
                    other.to_token_stream(),
                    ty.to_token_stream(),
                    attr_path,
                ),
            );
        }

        if method_context_ty.is_some() {
            let crate_path = mode.crate_path();

            parse_quote! {
                #trait_ident::#method_ident(self, #crate_path::FromContext::from(context))
            }
        } else {
            parse_quote! {
                #trait_ident::#method_ident(self)
            }
        }
    };

    // Doing this may be quite an expensive, because resolving may contain some heavy computation,
    // so we're preforming it twice. Unfortunately, we have no other options here, until the
    // `juniper::GraphQLType` itself will allow to do it in some cleverer way.
    let resolver_check = parse_quote! {
        ({ #resolver_code } as ::std::option::Option<&#ty>).is_some()
    };

    Some(UnionVariantDefinition {
        ty,
        resolver_code,
        resolver_check,
        enum_path: None,
        context_ty: method_context_ty,
        span: method_span,
    })
}

/// Parses type of [GraphQL union][1] variant from the return type of trait method.
///
/// If return type is invalid, then returns the [`Span`] to display the corresponding error at.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
fn parse_trait_method_output_type(sig: &syn::Signature) -> Result<syn::Type, Span> {
    let ret_ty = match &sig.output {
        syn::ReturnType::Type(_, ty) => ty.deref(),
        _ => return Err(sig.span()),
    };

    let path = match unparenthesize(ret_ty) {
        syn::Type::Path(syn::TypePath { qself: None, path }) => path,
        _ => return Err(ret_ty.span()),
    };

    let (ident, args) = match path.segments.last() {
        Some(syn::PathSegment {
            ident,
            arguments: syn::PathArguments::AngleBracketed(generic),
        }) => (ident, &generic.args),
        _ => return Err(ret_ty.span()),
    };

    if ident.unraw() != "Option" {
        return Err(ret_ty.span());
    }

    if args.len() != 1 {
        return Err(ret_ty.span());
    }
    let var_ty = match args.first() {
        Some(syn::GenericArgument::Type(inner_ty)) => match unparenthesize(inner_ty) {
            syn::Type::Reference(inner_ty) => {
                if inner_ty.mutability.is_some() {
                    return Err(inner_ty.span());
                }
                unparenthesize(inner_ty.elem.deref()).clone()
            }
            _ => return Err(ret_ty.span()),
        },
        _ => return Err(ret_ty.span()),
    };
    Ok(var_ty)
}

/// Parses trait method input arguments and validates them to be acceptable for resolving into
/// [GraphQL union][1] variant type. Returns type of the context used in input arguments, if any.
///
/// If input arguments are invalid, then returns the [`Span`] to display the corresponding error at.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
fn parse_trait_method_input_args(sig: &syn::Signature) -> Result<Option<syn::Type>, Span> {
    match sig.receiver() {
        Some(syn::FnArg::Receiver(rcv)) => {
            if rcv.reference.is_none() || rcv.mutability.is_some() {
                return Err(rcv.span());
            }
        }
        _ => return Err(sig.span()),
    }

    if sig.inputs.len() > 2 {
        return Err(sig.inputs.span());
    }

    let second_arg_ty = match sig.inputs.iter().skip(1).next() {
        Some(syn::FnArg::Typed(arg)) => arg.ty.deref(),
        None => return Ok(None),
        _ => return Err(sig.inputs.span()),
    };
    match unparenthesize(second_arg_ty) {
        syn::Type::Reference(ref_ty) => {
            if ref_ty.mutability.is_some() {
                return Err(ref_ty.span());
            }
            Ok(Some(ref_ty.elem.deref().clone()))
        }
        ty => Err(ty.span()),
    }
}

/// Checks whether all [GraphQL union][1] `variants` contains a different type of
/// `juniper::Context`.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
fn all_contexts_different(variants: &Vec<UnionVariantDefinition>) -> bool {
    let all: Vec<_> = variants
        .iter()
        .filter_map(|v| v.context_ty.as_ref())
        .collect();
    let deduped: HashSet<_> = variants
        .iter()
        .filter_map(|v| v.context_ty.as_ref())
        .collect();
    deduped.len() == all.len()
}
