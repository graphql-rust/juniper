use crate::{
    result::{GraphQLScope, UnsupportedAttribute},
    util::{self, span_container::SpanContainer, RenameRule},
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{self, ext::IdentExt, spanned::Spanned, Data, Fields};

pub fn build_derive_object(ast: syn::DeriveInput, error: GraphQLScope) -> syn::Result<TokenStream> {
    let ast_span = ast.span();
    let struct_fields = match ast.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => return Err(error.custom_error(ast_span, "only named fields are allowed")),
        },
        _ => return Err(error.custom_error(ast_span, "can only be applied to structs")),
    };

    // Parse attributes.
    let attrs = util::ObjectAttributes::from_attrs(&ast.attrs)?;

    let ident = &ast.ident;
    let name = attrs
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| ident.unraw().to_string());

    let fields = struct_fields
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

            if field_attrs.skip.is_some() {
                return None;
            }

            let field_name = &field.ident.unwrap();
            let name = field_attrs
                .name
                .clone()
                .map(SpanContainer::into_inner)
                .unwrap_or_else(|| {
                    attrs
                        .rename
                        .unwrap_or(RenameRule::CamelCase)
                        .apply(&field_name.unraw().to_string())
                });

            if name.starts_with("__") {
                error.no_double_underscore(if let Some(name) = field_attrs.name {
                    name.span_ident()
                } else {
                    field_name.span()
                });
            }

            if let Some(default) = field_attrs.default {
                error.unsupported_attribute_within(
                    default.span_ident(),
                    UnsupportedAttribute::Default,
                );
            }

            let resolver_code = quote!(
                &self . #field_name
            );

            Some(util::GraphQLTypeDefinitionField {
                name,
                _type: field.ty,
                args: Vec::new(),
                description: field_attrs.description.map(SpanContainer::into_inner),
                deprecation: field_attrs.deprecation.map(SpanContainer::into_inner),
                resolver_code,
                default: None,
                is_type_inferred: true,
                is_async: false,
                span,
            })
        })
        .collect::<Vec<_>>();

    // Early abort after checking all fields
    proc_macro_error::abort_if_dirty();

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

    if fields.is_empty() {
        error.not_empty(ast_span);
    }

    // Early abort after GraphQL properties
    proc_macro_error::abort_if_dirty();

    let definition = util::GraphQLTypeDefiniton {
        name,
        _type: syn::parse_str(&ast.ident.to_string()).unwrap(),
        context: attrs.context.map(SpanContainer::into_inner),
        scalar: attrs.scalar.map(SpanContainer::into_inner),
        description: attrs.description.map(SpanContainer::into_inner),
        fields,
        generics: ast.generics,
        interfaces: attrs
            .interfaces
            .into_iter()
            .map(SpanContainer::into_inner)
            .collect(),
        include_type_generics: true,
        generic_scalar: true,
        no_async: attrs.no_async.is_some(),
    };

    Ok(definition.into_tokens())
}
