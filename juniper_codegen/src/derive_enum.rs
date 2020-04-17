use proc_macro2::TokenStream;

use crate::util;
use quote::quote;
use syn::{self, Data, Fields};

pub fn impl_enum(ast: syn::DeriveInput, is_internal: bool) -> TokenStream {
    if !ast.generics.params.is_empty() {
        panic!("#[derive(GraphQLEnum) does not support generics or lifetimes");
    }

    let variants = match ast.data {
        Data::Enum(enum_data) => enum_data.variants,
        _ => {
            panic!("#[derive(GraphlQLEnum)] may only be applied to enums, not to structs");
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
        panic!("Invalid #[graphql(...)] attribute 'interfaces': #[derive(GraphQLEnum) does not support 'interfaces'");
    }
    if attrs.scalar.is_some() {
        panic!("Invalid #[graphql(...)] attribute 'scalar': #[derive(GraphQLEnum) does not support explicit scalars");
    }

    // Parse attributes.
    let ident = &ast.ident;
    let name = attrs.name.unwrap_or_else(|| ident.to_string());

    let mut mapping = std::collections::HashMap::new();

    let fields = variants
        .into_iter()
        .filter_map(|field| {
            let field_attrs = match util::FieldAttributes::from_attrs(
                field.attrs,
                util::FieldAttributeParseMode::Object,
            ) {
                Ok(attrs) => attrs,
                Err(e) => panic!("Invalid #[graphql] attribute for field: \n{}", e),
            };

            if field_attrs.skip {
                panic!("#[derive(GraphQLEnum)] does not support #[graphql(skip)] on fields");
            } else {
                let field_name = field.ident;
                let name = field_attrs
                    .name
                    .clone()
                    .unwrap_or_else(|| util::to_upper_snake_case(&field_name.to_string()));

                match mapping.get(&name) {
                    Some(other_field_name) =>
                        panic!(format!("#[derive(GraphQLEnum)] all variants needs to be unique. Another field name `{}` has the same identifier `{}`, thus `{}` can not be named `{}`. One of the fields is manually renamed!", other_field_name, name, field_name, name)),
                    None => {
                        mapping.insert(name.clone(), field_name.clone());
                    }
                }

                let resolver_code = quote!( #ident::#field_name );

                let _type = match field.fields {
                    Fields::Unit => syn::parse_str(&field_name.to_string()).unwrap(),
                    _ => panic!("#[derive(GraphQLEnum)] all fields of the enum must be unnamed"),
                };

                Some(util::GraphQLTypeDefinitionField {
                    name,
                    _type,
                    args: Vec::new(),
                    description: field_attrs.description,
                    deprecation: field_attrs.deprecation,
                    resolver_code,
                    is_type_inferred: true,
                    is_async: false,
                })
            }
        })
        .collect::<Vec<_>>();

    if fields.len() == 0 {
        panic!("#[derive(GraphQLEnum)] requires at least one variants");
    }

    let definition = util::GraphQLTypeDefiniton {
        name,
        _type: syn::parse_str(&ast.ident.to_string()).unwrap(),
        context: attrs.context,
        scalar: None,
        description: attrs.description,
        fields,
        // NOTICE: only unit variants allow -> no generics possible
        generics: syn::Generics::default(),
        interfaces: None,
        include_type_generics: true,
        generic_scalar: true,
        no_async: attrs.no_async,
    };

    let juniper_crate_name = if is_internal { "crate" } else { "juniper" };
    definition.into_enum_tokens(juniper_crate_name)
}
