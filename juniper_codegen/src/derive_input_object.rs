#![allow(clippy::match_wild_err_arm)]
use crate::{
    result::{GraphQLScope, UnsupportedAttribute},
    util::{self, span_container::SpanContainer, RenameRule},
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{self, ext::IdentExt, spanned::Spanned, Data, Fields};

pub fn impl_input_object(ast: syn::DeriveInput, error: GraphQLScope) -> syn::Result<TokenStream> {
    let ast_span = ast.span();
    let fields = match ast.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(named) => named.named,
            _ => {
                return Err(
                    error.custom_error(ast_span, "all fields must be named, e.g., `test: String`")
                )
            }
        },
        _ => return Err(error.custom_error(ast_span, "can only be used on structs with fields")),
    };

    // Parse attributes.
    let attrs = util::ObjectAttributes::from_attrs(&ast.attrs)?;

    // Parse attributes.
    let ident = &ast.ident;
    let name = attrs
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| ident.to_string());

    let fields = fields
        .into_iter()
        .filter_map(|field| {
            let span = field.span();
            let field_attrs = match util::FieldAttributes::from_attrs(
                &field.attrs,
                util::FieldAttributeParseMode::Object,
            ) {
                Ok(attrs) => attrs,
                Err(e) => {
                    proc_macro_error::emit_error!(e);
                    return None;
                }
            };

            let field_ident = field.ident.as_ref().unwrap();
            let name = match field_attrs.name {
                Some(ref name) => name.to_string(),
                None => attrs
                    .rename
                    .unwrap_or(RenameRule::CamelCase)
                    .apply(&field_ident.unraw().to_string()),
            };

            if let Some(span) = field_attrs.skip {
                error.unsupported_attribute_within(span.span(), UnsupportedAttribute::Skip)
            }

            if let Some(span) = field_attrs.deprecation {
                error.unsupported_attribute_within(
                    span.span_ident(),
                    UnsupportedAttribute::Deprecation,
                )
            }

            if name.starts_with("__") {
                error.no_double_underscore(if let Some(name) = field_attrs.name {
                    name.span_ident()
                } else {
                    name.span()
                });
            }

            let resolver_code = quote!(#field_ident);

            let default = field_attrs
                .default
                .map(|default| match default.into_inner() {
                    Some(expr) => expr.into_token_stream(),
                    None => quote! { Default::default() },
                });

            Some(util::GraphQLTypeDefinitionField {
                name,
                _type: field.ty,
                args: Vec::new(),
                description: field_attrs.description.map(SpanContainer::into_inner),
                deprecation: None,
                resolver_code,
                is_type_inferred: true,
                is_async: false,
                default,
                span,
            })
        })
        .collect::<Vec<_>>();

    proc_macro_error::abort_if_dirty();

    if fields.is_empty() {
        error.not_empty(ast_span);
    }

    if let Some(duplicates) =
        crate::util::duplicate::Duplicate::find_by_key(&fields, |field| &field.name)
    {
        error.duplicate(duplicates.iter())
    }

    if !attrs.interfaces.is_empty() {
        attrs.interfaces.iter().for_each(|elm| {
            error.unsupported_attribute(elm.span(), UnsupportedAttribute::Interface)
        });
    }

    if let Some(duplicates) =
        crate::util::duplicate::Duplicate::find_by_key(&fields, |field| field.name.as_str())
    {
        error.duplicate(duplicates.iter());
    }

    if !attrs.is_internal && name.starts_with("__") {
        error.no_double_underscore(if let Some(name) = attrs.name {
            name.span_ident()
        } else {
            ident.span()
        });
    }

    proc_macro_error::abort_if_dirty();

    let definition = util::GraphQLTypeDefiniton {
        name,
        _type: syn::parse_str(&ast.ident.to_string()).unwrap(),
        context: attrs.context.map(SpanContainer::into_inner),
        scalar: attrs.scalar.map(SpanContainer::into_inner),
        description: attrs.description.map(SpanContainer::into_inner),
        fields,
        generics: ast.generics,
        interfaces: vec![],
        include_type_generics: true,
        generic_scalar: true,
        no_async: attrs.no_async.is_some(),
    };

    Ok(definition.into_input_object_tokens())
}
