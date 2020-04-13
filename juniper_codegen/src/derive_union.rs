use proc_macro2::TokenStream;
use quote::quote;
use syn::{self, Data, Fields};

use crate::util;

pub fn build_derive_union(ast: syn::DeriveInput, is_internal: bool) -> TokenStream {
    let enum_fields = match ast.data {
        Data::Enum(data) => data.variants,
        _ => {
            panic!("#[derive(GraphlQLUnion)] can only be applied to enums");
        }
    };

    // Parse attributes.
    let attrs = match util::ObjectAttributes::from_attrs(&ast.attrs) {
        Ok(a) => a,
        Err(e) => {
            panic!("Invalid #[graphql(...)] attribute for enum: {}", e);
        }
    };

    if !attrs.interfaces.is_empty() {
        panic!("#[derive(GraphlQLUnion)] does not support interfaces");
    }

    let ident = &ast.ident;
    let name = attrs.name.unwrap_or_else(|| ident.to_string());

    let fields = enum_fields.into_iter().filter_map(|field| {
        let field_attrs = match util::FieldAttributes::from_attrs(
            field.attrs,
            util::FieldAttributeParseMode::Object,
        ) {
            Ok(attrs) => attrs,
            Err(e) => panic!("Invalid #[graphql] attribute for field: \n{}", e),
        };


        if field_attrs.skip {
            panic!("#[derive(GraphQLUnion)] does not support #[graphql(skip)] on fields");
        } else {
            let field_name = field.ident;
            let name = field_attrs
                .name
                .clone()
                .unwrap_or_else(|| util::to_camel_case(&field_name.to_string()));

            let resolver_code = quote!(
                #ident . #field_name
            );

            let _type = match field.fields {
                Fields::Unnamed(inner) => {
                    let mut iter = inner.unnamed.iter();
                    let first = match iter.next() {
                        Some(val) => val,
                        None => unreachable!(),
                    };

                    if iter.next().is_some() {
                        panic!("#[derive(GraphlQLUnion)] all members must be unnamed with a single element e.g. Some(T)");
                    }

                    first.ty.clone()
                }
                _ => panic!("#[derive(GraphlQLObject)] all fields of the enum must be unnamed"),
            };

            if field_attrs.description.is_some() {
                panic!("#[derive(GraphQLUnion)] does not allow documentation of fields");
            }

            Some(util::GraphQLTypeDefinitionField {
                name,
                _type,
                args: Vec::new(),
                description: None,
                deprecation: field_attrs.deprecation,
                resolver_code,
                is_type_inferred: true,
                is_async: false,
            })
        }
    });

    let definition = util::GraphQLTypeDefiniton {
        name,
        _type: syn::parse_str(&ast.ident.to_string()).unwrap(),
        context: attrs.context,
        scalar: attrs.scalar,
        description: attrs.description,
        fields: fields.collect(),
        generics: ast.generics,
        interfaces: None,
        include_type_generics: true,
        generic_scalar: true,
        no_async: attrs.no_async,
    };

    let juniper_crate_name = if is_internal { "crate" } else { "juniper" };
    definition.into_union_tokens(juniper_crate_name)
}
