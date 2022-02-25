//! Code generation for `#[graphql_scalar]` macro.

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{parse_quote, spanned::Spanned};

use crate::{
    common::{parse, scalar},
    graphql_scalar::TypeOrIdent,
    GraphQLScope,
};

use super::{Attr, Definition, Field, GraphQLScalarMethods, ParseToken};

const ERR: GraphQLScope = GraphQLScope::ScalarAttr;

/// Expands `#[graphql_scalar]` macro into generated code.
pub(crate) fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemType>(body.clone()) {
        let attrs = parse::attr::unite(("graphql_scalar", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip("graphql_scalar", ast.attrs);
        return expand_on_type_alias(attrs, ast);
    } else if let Ok(mut ast) = syn::parse2::<syn::DeriveInput>(body) {
        let attrs = parse::attr::unite(("graphql_scalar", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip("graphql_scalar", ast.attrs);
        return expand_on_derive_input(attrs, ast);
    }

    Err(syn::Error::new(
        Span::call_site(),
        "#[graphql_scalar] attribute is applicable to type aliases, structs, \
         enums and unions only",
    ))
}

/// Expands `#[graphql_scalar]` macro placed on a type alias.
fn expand_on_type_alias(
    attrs: Vec<syn::Attribute>,
    ast: syn::ItemType,
) -> syn::Result<TokenStream> {
    let attr = Attr::from_attrs("graphql_scalar", &attrs)?;

    if attr.transparent {
        return Err(ERR.custom_error(
            ast.span(),
            "`transparent` attribute isn't applicable for type aliases.",
        ));
    }

    let field = match (
        attr.to_output.as_deref().cloned(),
        attr.from_input.as_deref().cloned(),
        attr.parse_token.as_deref().cloned(),
        attr.with.as_deref().cloned(),
    ) {
        (Some(to_output), Some(from_input), Some(parse_token), None) => {
            GraphQLScalarMethods::Custom {
                to_output,
                from_input,
                parse_token,
            }
        }
        (to_output, from_input, parse_token, Some(module)) => GraphQLScalarMethods::Custom {
            to_output: to_output.unwrap_or_else(|| parse_quote! { #module::to_output }),
            from_input: from_input.unwrap_or_else(|| parse_quote! { #module::from_input }),
            parse_token: parse_token
                .unwrap_or_else(|| ParseToken::Custom(parse_quote! { #module::parse_token })),
        },
        _ => {
            return Err(ERR.custom_error(
                ast.span(),
                "all custom resolvers have to be provided via `with` or \
                 combination of `to_output_with`, `from_input_with`, \
                 `parse_token_with` attribute arguments",
            ));
        }
    };

    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    let def = Definition {
        ty: TypeOrIdent::Type(ast.ty.clone()),
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
    .to_token_stream();

    Ok(quote! {
        #ast
        #def
    })
}

/// Expands `#[graphql_scalar]` macro placed on a struct/enum/union.
fn expand_on_derive_input(
    attrs: Vec<syn::Attribute>,
    ast: syn::DeriveInput,
) -> syn::Result<TokenStream> {
    let attr = Attr::from_attrs("graphql_scalar", &attrs)?;

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

    let def = Definition {
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
    .to_token_stream();

    Ok(quote! {
        #ast
        #def
    })
}
