use std::ops::Deref as _;

use proc_macro2::{Span, TokenStream};
use proc_macro_error::ResultExt as _;
use quote::{quote, ToTokens as _};
use syn::{self, ext::IdentExt as _, parse_quote, spanned::Spanned as _};

use crate::{
    result::GraphQLScope,
    util::{span_container::SpanContainer, Mode},
};

use super::{UnionDefinition, UnionMeta, UnionVariantDefinition, UnionVariantMeta};

const SCOPE: GraphQLScope = GraphQLScope::AttrUnion;

/// Expands `#[graphql_union]` macro into generated code.
pub fn expand(attr: TokenStream, body: TokenStream, mode: Mode) -> syn::Result<TokenStream> {
    if !attr.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "#[graphql_union] attribute itself does not support any parameters, \
             use helper #[graphql] attributes instead to specify any parameters",
        ));
    }

    let ast = syn::parse2::<syn::ItemTrait>(body.clone()).map_err(|_| {
        syn::Error::new(
            Span::call_site(),
            "#[graphql_union] attribute is applicable to trait definitions only",
        )
    })?;

    let meta = UnionMeta::from_attrs(&ast.attrs)?;

    let trait_span = ast.span();
    let trait_ident = ast.ident;

    let name = meta
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| trait_ident.unraw().to_string()); // TODO: PascalCase
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
        .into_iter()
        .filter_map(|i| match i {
            syn::TraitItem::Method(m) => {
                parse_variant_from_trait_method(m, &trait_ident, &meta, mode)
            }
            _ => None,
        })
        .collect();

    proc_macro_error::abort_if_dirty();

    if !meta.custom_resolvers.is_empty() {
        let crate_path = mode.crate_path();
        // TODO: modify variants
    }
    if variants.is_empty() {
        SCOPE.custom(trait_span, "expects at least one union variant");
    }

    // NOTICE: This is not an optimal implementation, as it's possible to bypass this check by using
    // a full qualified path instead (`crate::Test` vs `Test`). Since this requirement is mandatory,
    // the `std::convert::Into<T>` implementation is used to enforce this requirement. However, due
    // to the bad error message this implementation should stay and provide guidance.
    let all_variants_different = {
        let mut types: Vec<_> = variants.iter().map(|var| &var.ty).collect();
        types.dedup();
        types.len() == variants.len()
    };
    if !all_variants_different {
        SCOPE.custom(trait_span, "each union variant must have a different type");
    }

    proc_macro_error::abort_if_dirty();

    let generated_code = UnionDefinition {
        name,
        ty: syn::parse_str(&trait_ident.to_string()).unwrap_or_abort(), // TODO: trait object
        description: meta.description.map(SpanContainer::into_inner),
        context: meta.context.map(SpanContainer::into_inner),
        scalar: meta.scalar.map(SpanContainer::into_inner),
        generics: ast.generics,
        variants,
        span: trait_span,
        mode,
    }
    .into_tokens();

    Ok(quote! {
        #body

        #generated_code
    })
}

fn parse_variant_from_trait_method(
    method: syn::TraitItemMethod,
    trait_ident: &syn::Ident,
    trait_meta: &UnionMeta,
    mode: Mode,
) -> Option<UnionVariantDefinition> {
    let meta = UnionVariantMeta::from_attrs(&method.attrs)
        .map_err(|e| proc_macro_error::emit_error!(e))
        .ok()?;
    if let Some(rslvr) = meta.custom_resolver {
        SCOPE.custom(
            rslvr.span_ident(),
            "cannot use #[graphql(with = ...)] attribute on a trait method, instead use \
             #[graphql(ignore)] on the method with #[graphql(on ... = ...)] on the trait itself",
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
                "trait method return type can be `Option<&VariantType>` only",
            )
        })
        .ok()?;
    let accepts_context = parse_trait_method_input_args(&method.sig)
        .map_err(|span| {
            SCOPE.custom(
                span,
                "trait method can accept `&self` and optionally `&Context` only",
            )
        })
        .ok()?;
    // TODO: validate signature to not be async

    let resolver_code = {
        if let Some(other) = trait_meta.custom_resolvers.get(&ty) {
            SCOPE.custom(
                method_span,
                format!(
                    "trait method `{}` conflicts with the custom resolver `{}` declared on the \
                     trait to resolve the variant type `{}`, use `#[graphql(ignore)]` attribute to \
                     ignore this trait method for union variants resolution",
                    method_ident,
                    other.to_token_stream(),
                    ty.to_token_stream(),
                ),
            );
        }

        if accepts_context {
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

    // Doing this may be quite an expensive, because resolving may contain some heavy
    // computation, so we're preforming it twice. Unfortunately, we have no other options
    // here, until the `juniper::GraphQLType` itself will allow to do it in some cleverer
    // way.
    let resolver_check = parse_quote! {
        ({ #resolver_code } as ::std::option::Option<&#ty>).is_some()
    };

    Some(UnionVariantDefinition {
        ty,
        resolver_code,
        resolver_check,
        enum_path: None,
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
/// [GraphQL union][1] variant type. Indicates whether method accepts context or not.
///
/// If input arguments are invalid, then returns the [`Span`] to display the corresponding error at.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
fn parse_trait_method_input_args(sig: &syn::Signature) -> Result<bool, Span> {
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
        None => return Ok(false),
        _ => return Err(sig.inputs.span()),
    };
    match unparenthesize(second_arg_ty) {
        syn::Type::Reference(ref_ty) => {
            if ref_ty.mutability.is_some() {
                return Err(ref_ty.span());
            }
        }
        ty => return Err(ty.span()),
    }

    Ok(true)
}

/// Retrieves the innermost non-parenthesized [`syn::Type`] from the given one.
fn unparenthesize(ty: &syn::Type) -> &syn::Type {
    match ty {
        syn::Type::Paren(ty) => unparenthesize(ty.elem.deref()),
        _ => ty,
    }
}
