use proc_macro2::TokenStream;
use proc_macro_error::ResultExt as _;
use quote::quote;
use syn::{self, ext::IdentExt, spanned::Spanned, Data, Fields};

use crate::{
    result::{GraphQLScope, UnsupportedAttribute},
    util::{self, span_container::SpanContainer, Mode},
};

const SCOPE: GraphQLScope = GraphQLScope::DeriveUnion;

pub fn expand(input: TokenStream, mode: Mode) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input).unwrap_or_abort();
    let ast_span = ast.span();

    let enum_fields = match ast.data {
        Data::Enum(data) => data.variants,
        Data::Struct(_) => unimplemented!(),
        _ => return Err(SCOPE.custom_error(ast_span, "can only be applied to enums and structs")),
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
                SCOPE.unsupported_attribute_within(ident.span(), UnsupportedAttribute::Skip);
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
                    let first = iter.next().unwrap();

                    if iter.next().is_some() {
                        SCOPE.custom(
                            inner.span(),
                            "all members must be unnamed with a single element e.g. Some(T)",
                        );
                    }

                    first.ty.clone()
                }
                _ => {
                    SCOPE.custom(
                        variant_name.span(),
                        "only unnamed fields with a single element are allowed, e.g., Some(T)",
                    );

                    return None;
                }
            };

            if let Some(description) = field_attrs.description {
                SCOPE.unsupported_attribute_within(
                    description.span_ident(),
                    UnsupportedAttribute::Description,
                );
            }

            if let Some(default) = field_attrs.default {
                SCOPE.unsupported_attribute_within(
                    default.span_ident(),
                    UnsupportedAttribute::Default,
                );
            }

            if name.starts_with("__") {
                SCOPE.no_double_underscore(if let Some(name) = field_attrs.name {
                    name.span_ident()
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
            SCOPE.unsupported_attribute(elm.span(), UnsupportedAttribute::Interface)
        });
    }

    if fields.is_empty() {
        SCOPE.not_empty(ast_span);
    }

    if name.starts_with("__") && matches!(mode, Mode::Public) {
        SCOPE.no_double_underscore(if let Some(name) = attrs.name {
            name.span_ident()
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
        SCOPE.custom(ident.span(), "each variant must have a different type");
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
        mode,
    };

    Ok(definition.into_union_tokens())
}
