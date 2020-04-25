use crate::result::{Generator, UnsupportedAttribute};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{self, spanned::Spanned, Data, Fields};

use crate::util;

pub fn build_derive_union(
    ast: syn::DeriveInput,
    is_internal: bool,
    error: Generator,
) -> TokenStream {
    let span = ast.span();
    let enum_fields = match ast.data {
        Data::Enum(data) => data.variants,
        _ => return error.custom(ast.ident.span(), "only enums are supported"),
    };

    // Parse attributes.
    let attrs = match util::ObjectAttributes::from_attrs(&ast.attrs) {
        Ok(a) => a,
        Err(e) => {
            panic!("Invalid #[graphql(...)] attribute for enum: {}", e);
        }
    };

    if !attrs.interfaces.is_empty() {
        return attrs
            .interfaces
            .iter()
            .map(|elm| error.unsupported_attribute(elm.span(), UnsupportedAttribute::Interface))
            .collect();
    }

    let ident = &ast.ident;
    let name = attrs.name.unwrap_or_else(|| ident.to_string());

    let fields = enum_fields.into_iter().filter_map(|field| {
        let span = field.span();
        let field_attrs = match util::FieldAttributes::from_attrs(
            &field.attrs,
            util::FieldAttributeParseMode::Object,
        ) {
            Ok(attrs) => attrs,
            Err(e) => return Some(Err(e.to_compile_error())),
        };

        if let Some(ident) = field_attrs.skip {
            return Some(Err(
                error.unsupported_attribute(ident.span(), UnsupportedAttribute::Skip)
            ));
        } else {
            let variant_name = field.ident;
            let name = field_attrs
                .name
                .clone()
                .unwrap_or_else(|| util::to_camel_case(&variant_name.to_string()));

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
                        return Some(Err(error.custom(
                            inner.span(),
                            "all members must be unnamed with a single element e.g. Some(T)",
                        )));
                    }

                    first.ty.clone()
                }
                _ => {
                    return Some(Err(
                        error.custom(variant_name.span(), "only unnamed fields are allowed")
                    ))
                }
            };

            if field_attrs.description.is_some() {
                // return Some(Err(
                //     context.unsupported_attribute(, UnsupportedAttribute::Interface)
                // ));
            }

            Some(Ok(util::GraphQLTypeDefinitionField {
                name,
                _type,
                args: Vec::new(),
                description: None,
                deprecation: field_attrs.deprecation,
                resolver_code,
                is_type_inferred: true,
                is_async: false,
                span,
            }))
        }
    });

    let fields: Vec<_> = match fields.collect() {
        Ok(fields) => fields,
        Err(tokens) => return tokens,
    };

    if fields.is_empty() {
        return error.not_empty(span);
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
        return error.custom(ident.span(), "each variant must have a different type");
    }

    let definition = util::GraphQLTypeDefiniton {
        name,
        _type: syn::parse_str(&ast.ident.to_string()).unwrap(),
        context: attrs.context,
        scalar: attrs.scalar,
        description: attrs.description,
        fields,
        generics: ast.generics,
        interfaces: None,
        include_type_generics: true,
        generic_scalar: true,
        no_async: attrs.no_async,
    };

    let juniper_crate_name = if is_internal { "crate" } else { "juniper" };
    definition.into_union_tokens(juniper_crate_name)
}
