use proc_macro2::TokenStream;
use quote::quote;
use syn::{self, Data, Fields};

use crate::util;

pub fn build_derive_object(ast: syn::DeriveInput, is_internal: bool) -> TokenStream {
    let struct_fields = match ast.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => {
                panic!("#[derive(GraphQLObject)] may only be used on regular structs with fields");
            }
        },
        _ => {
            panic!("#[derive(GraphlQLObject)] may only be applied to structs, not to enums");
        }
    };

    // Parse attributes.
    let attrs = match util::ObjectAttributes::from_attrs(&ast.attrs) {
        Ok(a) => a,
        Err(e) => {
            panic!("Invalid #[graphql(...)] attribute: {}", e);
        }
    };
    if !attrs.interfaces.is_empty() {
        panic!("Invalid #[graphql(...)] attribute 'interfaces': #[derive(GraphQLObject) does not support 'interfaces'");
    }
    let ident = &ast.ident;
    let name = attrs.name.unwrap_or_else(|| ident.to_string());

    let fields = struct_fields.into_iter().filter_map(|field| {
        let field_attrs = match util::FieldAttributes::from_attrs(
            field.attrs,
            util::FieldAttributeParseMode::Object,
        ) {
            Ok(attrs) => attrs,
            Err(e) => panic!("Invalid #[graphql] attribute: \n{}", e),
        };

        if field_attrs.skip {
            None
        } else {
            let field_name = field.ident.unwrap();
            let name = field_attrs
                .name
                .clone()
                .unwrap_or_else(|| util::to_camel_case(&field_name.to_string()));

            let resolver_code = quote!(
                &self . #field_name
            );

            Some(util::GraphQLTypeDefinitionField {
                name,
                _type: field.ty,
                args: Vec::new(),
                description: field_attrs.description,
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
    definition.into_tokens(juniper_crate_name)
}
