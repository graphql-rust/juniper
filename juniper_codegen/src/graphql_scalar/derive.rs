//! Code generation for `#[derive(GraphQLScalar)]` macro.

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse_quote, spanned::Spanned};

use crate::common::{diagnostic, scalar, SpanContainer};

use super::{Attr, Definition, Field, Methods, ParseToken, TypeOrIdent};

/// [`diagnostic::Scope`] of errors for `#[derive(GraphQLScalar)]` macro.
const ERR: diagnostic::Scope = diagnostic::Scope::ScalarDerive;

/// Expands `#[derive(GraphQLScalar)]` macro into generated code.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input)?;
    let attr = Attr::from_attrs("graphql", &ast.attrs)?;
    let methods = parse_derived_methods(&ast, &attr)?;
    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    Ok(Definition {
        ty: TypeOrIdent::Ident(ast.ident.clone()),
        where_clause: attr
            .where_clause
            .map_or_else(Vec::new, |cl| cl.into_inner()),
        generics: ast.generics.clone(),
        methods,
        name: attr
            .name
            .map(SpanContainer::into_inner)
            .unwrap_or_else(|| ast.ident.to_string()),
        description: attr.description.map(SpanContainer::into_inner),
        specified_by_url: attr.specified_by_url.map(SpanContainer::into_inner),
        scalar,
    }
    .to_token_stream())
}

/// Parses [`Methods`] from the provided [`Attr`] for the specified
/// [`syn::DeriveInput`].
pub(super) fn parse_derived_methods(ast: &syn::DeriveInput, attr: &Attr) -> syn::Result<Methods> {
    match (
        attr.to_output.as_deref().cloned(),
        attr.from_input.as_deref().cloned(),
        attr.parse_token.as_deref().cloned(),
        attr.with.as_deref().cloned(),
        attr.transparent,
    ) {
        (Some(to_output), Some(from_input), Some(parse_token), None, false) => {
            Ok(Methods::Custom {
                to_output,
                from_input,
                parse_token,
            })
        }
        (to_output, from_input, parse_token, module, false) => {
            let module = module.unwrap_or_else(|| parse_quote! { Self });
            Ok(Methods::Custom {
                to_output: to_output.unwrap_or_else(|| parse_quote! { #module::to_output }),
                from_input: from_input.unwrap_or_else(|| parse_quote! { #module::from_input }),
                parse_token: parse_token
                    .unwrap_or_else(|| ParseToken::Custom(parse_quote! { #module::parse_token })),
            })
        }
        (to_output, from_input, parse_token, None, true) => {
            let data = if let syn::Data::Struct(data) = &ast.data {
                data
            } else {
                return Err(ERR.custom_error(
                    ast.span(),
                    "`transparent` attribute argument requires exactly 1 field",
                ));
            };
            let field = match &data.fields {
                syn::Fields::Unit => Err(ERR.custom_error(
                    ast.span(),
                    "`transparent` attribute argument requires exactly 1 field",
                )),
                syn::Fields::Unnamed(fields) => fields
                    .unnamed
                    .first()
                    .filter(|_| fields.unnamed.len() == 1)
                    .cloned()
                    .map(Field::Unnamed)
                    .ok_or_else(|| {
                        ERR.custom_error(
                            ast.span(),
                            "`transparent` attribute argument requires \
                             exactly 1 field",
                        )
                    }),
                syn::Fields::Named(fields) => fields
                    .named
                    .first()
                    .filter(|_| fields.named.len() == 1)
                    .cloned()
                    .map(Field::Named)
                    .ok_or_else(|| {
                        ERR.custom_error(
                            ast.span(),
                            "`transparent` attribute argument requires \
                             exactly 1 field",
                        )
                    }),
            }?;
            Ok(Methods::Delegated {
                to_output,
                from_input,
                parse_token,
                field: Box::new(field),
            })
        }
        (_, _, _, Some(module), true) => Err(ERR.custom_error(
            module.span(),
            "`with = <path>` attribute argument cannot be combined with \
             `transparent`. \
             You can specify custom resolvers with `to_output_with`, \
             `from_input_with`, `parse_token`/`parse_token_with` attribute \
             arguments and still use `transparent` for unspecified ones.",
        )),
    }
}
