use proc_macro2::{Span, TokenStream};
use quote::ToTokens as _;
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned as _};

use crate::{
    common::{parse, scalar},
    util::span_container::SpanContainer,
    GraphQLScope,
};

use super::{Attr, Definition};

/// [`GraphQLScope`] of errors for `#[graphql_scalar]` macro.
const ERR: GraphQLScope = GraphQLScope::ImplScalar;

/// Expands `#[graphql_scalar]` macro into generated code.
pub(crate) fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemImpl>(body) {
        let attrs = parse::attr::unite(("graphql_scalar", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip("graphql_scalar", ast.attrs);
        return expand_on_impl(attrs, ast);
    }

    Err(syn::Error::new(
        Span::call_site(),
        "#[graphql_scalar] attribute is applicable to impl trait block only",
    ))
}

/// Expands `#[graphql_scalar]` macro placed on an implementation block.
fn expand_on_impl(attrs: Vec<syn::Attribute>, ast: syn::ItemImpl) -> syn::Result<TokenStream> {
    let attr = Attr::from_attrs("graphql_scalar", &attrs)?;

    let mut self_ty = ast.self_ty.clone();
    if let syn::Type::Group(group) = self_ty.as_ref() {
        self_ty = group.elem.clone();
    }

    let name = attr
        .name
        .map(SpanContainer::into_inner)
        .or_else(|| {
            if let syn::Type::Path(path) = self_ty.as_ref() {
                path.path
                    .segments
                    .last()
                    .map(|last| last.ident.unraw().to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            ERR.custom_error(
                self_ty.span(),
                "unable to find target for implementation target for `GraphQLScalar`",
            )
        })?;

    let (_, trait_ty, _) = ast.trait_.as_ref().ok_or_else(|| {
        ERR.custom_error(
            ast.impl_token.span(),
            "expected `GraphQLScalar` trait implementation",
        )
    })?;

    let scalar = get_scalar(trait_ty, &ast.generics);

    let mut out = ast.to_token_stream();
    Definition {
        impl_for_type: *ast.self_ty.clone(),
        generics: ast.generics.clone(),
        name,
        description: attr.description.as_deref().cloned(),
        scalar,
        specified_by_url: attr.specified_by_url.as_deref().cloned(),
    }
    .to_tokens(&mut out);

    Ok(out)
}

/// Extracts [`scalar::Type`] from [`GraphQLScalar`] trait.
///
/// [`GraphQLScalar`]: juniper::GraphQLScalar
fn get_scalar(trait_ty: &syn::Path, generics: &syn::Generics) -> scalar::Type {
    if let Some(last_seg) = trait_ty.segments.last() {
        match &last_seg.arguments {
            syn::PathArguments::AngleBracketed(gens) => {
                if let Some(syn::GenericArgument::Type(ty)) = gens.args.last() {
                    let generic_scalar = generics
                        .params
                        .iter()
                        .filter_map(|par| match par {
                            syn::GenericParam::Type(ty) => Some(&ty.ident),
                            _ => None,
                        })
                        .find(|gen_par| gen_par.to_string() == ty.to_token_stream().to_string());

                    return generic_scalar.map_or_else(
                        || scalar::Type::Concrete(ty.clone()),
                        |scalar| scalar::Type::ExplicitGeneric(scalar.clone()),
                    );
                }
            }
            syn::PathArguments::None | syn::PathArguments::Parenthesized(_) => {}
        }
    }
    scalar::Type::Concrete(parse_quote! { ::juniper::DefaultScalarValue })
}
