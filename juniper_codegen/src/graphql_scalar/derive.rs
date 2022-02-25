//! Code generation for `#[derive(GraphQLScalar)]` macro.

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse_quote, spanned::Spanned};

use crate::{common::scalar, result::GraphQLScope};

use super::{Attr, Definition, Field, GraphQLScalarMethods, ParseToken, TypeOrIdent};

/// [`GraphQLScope`] of errors for `#[derive(GraphQLScalar)]` macro.
const ERR: GraphQLScope = GraphQLScope::ScalarDerive;

/// Expands `#[derive(GraphQLScalar)]` macro into generated code.
pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input)?;

    let attr = Attr::from_attrs("graphql", &ast.attrs)?;

    let field = match (
        attr.to_output.as_deref().cloned(),
        attr.from_input.as_deref().cloned(),
        attr.parse_token.as_deref().cloned(),
        attr.with.as_deref().cloned(),
        attr.transparent,
    ) {
        (Some(to_output), Some(from_input), Some(parse_token), None, false) => {
            GraphQLScalarMethods::Custom {
                to_output,
                from_input,
                parse_token,
            }
        }
        (to_output, from_input, parse_token, module, false) => {
            let module = module.unwrap_or_else(|| parse_quote! { Self });
            GraphQLScalarMethods::Custom {
                to_output: to_output.unwrap_or_else(|| parse_quote! { #module::to_output }),
                from_input: from_input.unwrap_or_else(|| parse_quote! { #module::from_input }),
                parse_token: parse_token
                    .unwrap_or_else(|| ParseToken::Custom(parse_quote! { #module::parse_token })),
            }
        }
        (to_output, from_input, parse_token, None, true) => {
            let data = if let syn::Data::Struct(data) = &ast.data {
                data
            } else {
                return Err(ERR.custom_error(
                    ast.span(),
                    "expected single-field struct because of `transparent` attribute",
                ));
            };
            let field = match &data.fields {
                syn::Fields::Unit => Err(ERR.custom_error(
                    ast.span(),
                    "expected exactly 1 field, e.g.: `Test(i32)`, `Test { test: i32 }` \
                     because of `transparent` attribute",
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
                            "expected exactly 1 field, e.g., Test(i32) \
                             because of `transparent` attribute",
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
                            "expected exactly 1 field, e.g., Test { test: i32 } \
                             because of `transparent` attribute",
                        )
                    }),
            }?;
            GraphQLScalarMethods::Delegated {
                to_output,
                from_input,
                parse_token,
                field: Box::new(field),
            }
        }
        (_, _, _, Some(module), true) => {
            return Err(ERR.custom_error(
                module.span(),
                "`with = <path>` attribute can't be combined with `transparent`. \
                 You can specify custom resolvers with `to_output`, `from_input`, `parse_token` \
                 attributes and still use `transparent` for unspecified ones.",
            ));
        }
    };

    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    Ok(Definition {
        ty: TypeOrIdent::Ident(ast.ident.clone()),
        where_clause: attr
            .where_clause
            .map_or_else(Vec::new, |cl| cl.into_inner()),
        generics: ast.generics.clone(),
        methods: field,
        name: attr
            .name
            .as_deref()
            .cloned()
            .unwrap_or_else(|| ast.ident.to_string()),
        description: attr.description.as_deref().cloned(),
        specified_by_url: attr.specified_by_url.as_deref().cloned(),
        scalar,
    }
    .to_token_stream())
}
