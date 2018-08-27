use proc_macro2::TokenStream;
use syn;
use syn::{Data, DeriveInput, Field, Fields};

use util::*;

#[derive(Default, Debug)]
struct ObjAttrs {
    name: Option<String>,
    description: Option<String>,
}

impl ObjAttrs {
    fn from_input(input: &DeriveInput) -> ObjAttrs {
        let mut res = ObjAttrs::default();

        // Check doc comments for description.
        res.description = get_doc_comment(&input.attrs);

        // Check attributes for name and description.
        if let Some(items) = get_graphql_attr(&input.attrs) {
            for item in items {
                if let Some(AttributeValue::String(val)) = keyed_item_value(&item, "name", AttributeValidation::String)  {
                    if is_valid_name(&*val) {
                        res.name = Some(val);
                        continue;
                    } else {
                        panic!(
                            "Names must match /^[_a-zA-Z][_a-zA-Z0-9]*$/ but \"{}\" does not",
                            &*val
                        );
                    }
                }
                if let Some(AttributeValue::String(val)) = keyed_item_value(&item, "description", AttributeValidation::String)  {
                    res.description = Some(val);
                    continue;
                }
                panic!(format!(
                    "Unknown object attribute for #[derive(GraphQLObject)]: {:?}",
                    item
                ));
            }
        }
        res
    }
}

#[derive(Default)]
struct ObjFieldAttrs {
    name: Option<String>,
    description: Option<String>,
    deprecation: Option<String>,
    skip: bool,
}

impl ObjFieldAttrs {
    fn from_input(variant: &Field) -> ObjFieldAttrs {
        let mut res = ObjFieldAttrs::default();

        // Check doc comments for description.
        res.description = get_doc_comment(&variant.attrs);

        // Check attributes.
        if let Some(items) = get_graphql_attr(&variant.attrs) {
            for item in items {
                if let Some(AttributeValue::String(val)) = keyed_item_value(&item, "name", AttributeValidation::String)  {
                    if is_valid_name(&*val) {
                        res.name = Some(val);
                        continue;
                    } else {
                        panic!(
                            "Names must match /^[_a-zA-Z][_a-zA-Z0-9]*$/ but \"{}\" does not",
                            &*val
                        );
                    }
                }
                if let Some(AttributeValue::String(val)) = keyed_item_value(&item, "description", AttributeValidation::String)  {
                    res.description = Some(val);
                    continue;
                }
                if let Some(AttributeValue::String(val)) = keyed_item_value(&item, "deprecation", AttributeValidation::String) {
                    res.deprecation = Some(val);
                    continue;
                }
                if let Some(_) = keyed_item_value(&item, "skip", AttributeValidation::Bare) {
                    res.skip = true;
                    continue;
                }
                panic!(format!(
                    "Unknown field attribute for #[derive(GraphQLObject)]: {:?}",
                    item
                ));
            }
        }
        res
    }
}

pub fn impl_object(ast: &syn::DeriveInput) -> TokenStream {
    let fields = match ast.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => fields.named.iter().collect::<Vec<_>>(),
            _ => {
                panic!("#[derive(GraphQLObject)] may only be used on regular structs with fields");
            }
        },
        _ => {
            panic!("#[derive(GraphlQLObject)] may only be applied to structs, not to enums");
        }
    };

    // Parse attributes.
    let ident = &ast.ident;
    let generics = &ast.generics;
    let ident_name = ident.to_string();
    let attrs = ObjAttrs::from_input(ast);
    let name = attrs.name.unwrap_or(ast.ident.to_string());
    let build_description = match attrs.description {
        Some(s) => quote!{ builder.description(#s)  },
        None => quote!{ builder },
    };

    let mut meta_fields = TokenStream::new();
    let mut resolvers = TokenStream::new();

    for field in fields {
        let field_ty = &field.ty;
        let field_attrs = ObjFieldAttrs::from_input(field);
        let field_ident = field.ident.as_ref().unwrap();

        // Check if we should skip this field.
        if field_attrs.skip {
            continue;
        }

        // Build value.
        let name = match field_attrs.name {
            Some(ref name) => {
                // Custom name specified.
                name.to_string()
            }
            None => {
                // Note: auto camel casing when no custom name specified.
                ::util::to_camel_case(&field_ident.to_string())
            }
        };
        let build_description = match field_attrs.description {
            Some(s) => quote!{ field.description(#s)  },
            None => quote!{ field },
        };

        let build_deprecation = match field_attrs.deprecation {
            Some(s) => quote!{ field.deprecated(#s)  },
            None => quote!{ field },
        };

        meta_fields.extend(quote!{
            {
                let field = registry.field::<#field_ty>(#name, &());
                let field = #build_description;
                let field = #build_deprecation;
                field
            },
        });

        // Build from_input clause.

        resolvers.extend(quote!{
            #name => executor.resolve_with_ctx(&(), &self.#field_ident),
        });
    }

    let toks = quote! {
        impl #generics ::juniper::GraphQLType for #ident #generics {
            type Context = ();
            type TypeInfo = ();

            fn name(_: &()) -> Option<&str> {
                Some(#name)
            }

            fn concrete_type_name(&self, _: &Self::Context, _: &()) -> String {
                #name.to_string()
            }

            fn meta<'r>(
                _: &(),
                registry: &mut ::juniper::Registry<'r>
            ) -> ::juniper::meta::MetaType<'r> {
                let fields = &[
                    #(#meta_fields)*
                ];
                let builder = registry.build_object_type::<#ident>(&(), fields);
                let builder = #build_description;
                builder.into_meta()
            }

            fn resolve_field(
                &self,
                _: &(),
                field_name: &str,
                _: &::juniper::Arguments,
                executor: &::juniper::Executor<Self::Context>
            ) -> ::juniper::ExecutionResult
            {

                match field_name {
                    #(#resolvers)*
                    _ => panic!("Field {} not found on type {}", field_name, #ident_name),
                }

            }
        }
    };

    toks
}
