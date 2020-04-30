use crate::{
    result::{GraphQLScope, UnsupportedAttribute},
    util::{self, span_container::SpanContainer},
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{self, ext::IdentExt, spanned::Spanned, Data, Fields};

pub fn build_derive_union(
    ast: syn::DeriveInput,
    is_internal: bool,
    error: GraphQLScope,
) -> syn::Result<TokenStream> {
    let ast_span = ast.span();
    let enum_fields = match ast.data {
        Data::Enum(data) => data.variants,
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

    let fields = enum_fields
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

            if let Some(ident) = field_attrs.skip {
                error.unsupported_attribute_within(ident.span(), UnsupportedAttribute::Skip);
                return None;
            }

            let variant_name = field.ident;
            let name = field_attrs
                .name
                .clone()
                .map(SpanContainer::into_inner)
                .unwrap_or_else(|| util::to_camel_case(&variant_name.unraw().to_string()));

            let resolver_code = quote!(
                #ident :: #variant_name
            );

            let _type = match field.fields {
                Fields::Unnamed(inner) => {
                    let mut iter = inner.unnamed.iter();
                    let first = match iter.next() {
                        Some(val) => val,
                        None => unreachable!(),
                    };

                    if iter.next().is_some() {
                        error.custom(
                            inner.span(),
                            "all members must be unnamed with a single element e.g. Some(T)",
                        );
                    }

                    first.ty.clone()
                }
                _ => {
                    error.custom(
                        variant_name.span(),
                        "only unnamed fields with a single element are allowed, e.g., Some(T)",
                    );

                    return None;
                }
            };

            if let Some(description) = field_attrs.description {
                error.unsupported_attribute_within(
                    description.span_ident(),
                    UnsupportedAttribute::Description,
                );
            }

            if let Some(default) = field_attrs.default {
                error.unsupported_attribute_within(
                    default.span_ident(),
                    UnsupportedAttribute::Default,
                );
            }

            if name.starts_with("__") {
                error.no_double_underscore(if let Some(name) = field_attrs.name {
                    name.span()
                } else {
                    variant_name.span()
                });
            }

            Some(util::GraphQLTypeDefinitionField {
                name,
                _type,
                args: Vec::new(),
                description: None,
                deprecation: field_attrs.deprecation.map(SpanContainer::into_inner),
                resolver_code,
                is_type_inferred: true,
                is_async: false,
                default: None,
                span,
            })
        })
        .collect::<Vec<_>>();

    // Early abort after checking all fields
    proc_macro_error::abort_if_dirty();

    if !attrs.interfaces.is_empty() {
        attrs.interfaces.iter().for_each(|elm| {
            error.unsupported_attribute(elm.span(), UnsupportedAttribute::Interface)
        });
    }

    if fields.is_empty() {
        error.not_empty(ast_span);
    }

    if name.starts_with("__") && !is_internal {
        error.no_double_underscore(if let Some(name) = attrs.name {
            name.span()
        } else {
            ident.span()
        });
    }

    // NOTICE: This is not an optimal implementation. It is possible
    // to bypass this check by using a full qualified path instead
    // (crate::Test vs Test). Since this requirement is mandatory, the
    // `std::convert::Into<T>` implementation is used to enforce this
    // requirement. However, due to the bad error message this
    // implementation should stay and provide guidance.
    let all_variants_different = {
        let mut all_types: Vec<_> = fields.iter().map(|field| &field._type).collect();
        let before = all_types.len();
        all_types.dedup();
        before == all_types.len()
    };

    if !all_variants_different {
        error.custom(ident.span(), "each variant must have a different type");
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
        interfaces: None,
        include_type_generics: true,
        generic_scalar: true,
        no_async: attrs.no_async.is_some(),
    };

    let juniper_crate_name = if is_internal { "crate" } else { "juniper" };
    Ok(definition.into_union_tokens(juniper_crate_name))
}
