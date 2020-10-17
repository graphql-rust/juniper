use proc_macro2::TokenStream;
use quote::quote;
use syn::{ext::IdentExt, spanned::Spanned, Data, Fields};

use crate::{
    result::{GraphQLScope, UnsupportedAttribute},
    util::{self, span_container::SpanContainer, RenameRule},
};

pub fn impl_enum(ast: syn::DeriveInput, error: GraphQLScope) -> syn::Result<TokenStream> {
    let ast_span = ast.span();

    if !ast.generics.params.is_empty() {
        return Err(error.custom_error(ast_span, "does not support generics or lifetimes"));
    }

    let variants = match ast.data {
        Data::Enum(enum_data) => enum_data.variants,
        _ => return Err(error.custom_error(ast_span, "can only be applied to enums")),
    };

    // Parse attributes.
    let attrs = util::ObjectAttributes::from_attrs(&ast.attrs)?;
    let ident = &ast.ident;
    let name = attrs
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| ident.unraw().to_string());

    let fields = variants
        .into_iter()
        .filter_map(|field| {
            let span = field.span();
            let field_attrs = match util::FieldAttributes::from_attrs(
                &field.attrs,
                util::FieldAttributeParseMode::Object,
            ) {
                Ok(attrs) => attrs,
                Err(err) => {
                    proc_macro_error::emit_error!(err);
                    return None;
                }
            };

            let field_name = field.ident;
            let name = field_attrs
                .name
                .clone()
                .map(SpanContainer::into_inner)
                .unwrap_or_else(|| {
                    attrs
                        .rename
                        .unwrap_or(RenameRule::ScreamingSnakeCase)
                        .apply(&field_name.unraw().to_string())
                });

            let resolver_code = quote!( #ident::#field_name );

            let _type = match field.fields {
                Fields::Unit => syn::parse_str(&field_name.to_string()).unwrap(),
                _ => {
                    error.emit_custom(
                        field.fields.span(),
                        "all fields of the enum must be unnamed, e.g., None",
                    );
                    return None;
                }
            };

            if let Some(skip) = field_attrs.skip {
                error.unsupported_attribute(skip.span(), UnsupportedAttribute::Skip);
                return None;
            }

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

            Some(util::GraphQLTypeDefinitionField {
                name,
                _type,
                args: Vec::new(),
                description: field_attrs.description.map(SpanContainer::into_inner),
                deprecation: field_attrs.deprecation.map(SpanContainer::into_inner),
                resolver_code,
                is_type_inferred: true,
                is_async: false,
                default: None,
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

    if let Some(scalar) = attrs.scalar {
        error.unsupported_attribute(scalar.span_ident(), UnsupportedAttribute::Scalar);
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
        scalar: None,
        description: attrs.description.map(SpanContainer::into_inner),
        fields,
        // NOTICE: only unit variants allow -> no generics possible
        generics: syn::Generics::default(),
        interfaces: vec![],
        include_type_generics: true,
        generic_scalar: true,
        no_async: attrs.no_async.is_some(),
    };

    Ok(definition.into_enum_tokens())
}
