use syn;
use syn::{
    DeriveInput,
    Data,
    Fields,
    Field,
};
use quote::Tokens;

use util::*;

#[derive(Default, Debug)]
struct ObjAttrs {
    name: Option<String>,
    description: Option<String>,
}

impl ObjAttrs {
    fn from_input(input: &DeriveInput) -> ObjAttrs {
        let mut res = ObjAttrs::default();

        // Check attributes for name and description.
        if let Some(items) = get_graphl_attr(&input.attrs) {
            for item in items {
                if let Some(val) = keyed_item_value(&item, "name", true) {
                    if is_valid_name(&*val) {
                        res.name = Some(val);
                        continue;
                    } else {
                         panic!("Names must match /^[_a-zA-Z][_a-zA-Z0-9]*$/ but \"{}\" does not",
                                 &*val);
                    }
                }
                if let Some(val) = keyed_item_value(&item, "description", true) {
                    res.description = Some(val);
                    continue;
                }
                panic!(format!(
                    "Unknown attribute for #[derive(GraphQLObject)]: {:?}",
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
}

impl ObjFieldAttrs {
    fn from_input(variant: &Field) -> ObjFieldAttrs {
        let mut res = ObjFieldAttrs::default();

        // Check attributes for name and description.
        if let Some(items) = get_graphl_attr(&variant.attrs) {
            for item in items {
                if let Some(val) = keyed_item_value(&item, "name", true) {
                    if is_valid_name(&*val) {
                        res.name = Some(val);
                        continue;
                    } else {
                         panic!("Names must match /^[_a-zA-Z][_a-zA-Z0-9]*$/ but \"{}\" does not",
                                 &*val);
                    }
                }
                if let Some(val) = keyed_item_value(&item, "description", true) {
                    res.description = Some(val);
                    continue;
                }
                if let Some(val) = keyed_item_value(&item, "deprecation", true) {
                    res.deprecation = Some(val);
                    continue;
                }
                panic!(format!(
                    "Unknown attribute for #[derive(GraphQLObject)]: {:?}",
                    item
                ));
            }
        }
        res
    }
}

pub fn impl_object(ast: &syn::DeriveInput) -> Tokens {
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
    let ident_name = ident.to_string();
    let attrs = ObjAttrs::from_input(ast);
    let name = attrs.name.unwrap_or(ast.ident.to_string());
    let build_description = match attrs.description {
        Some(s) => quote!{ builder.description(#s)  },
        None => quote!{ builder },
    };

    let mut meta_fields = Vec::<Tokens>::new();
    let mut resolvers = Vec::<Tokens>::new();

    for field in fields {
        let field_ty = &field.ty;
        let field_attrs = ObjFieldAttrs::from_input(field);
        let field_ident = field.ident.as_ref().unwrap();

        // Build value.
        let name = match field_attrs.name {
            Some(ref name) => {
                // Custom name specified.
                name.to_string()
            }
            None => {
                // Note: auto camel casing when no custom name specified.
                ::util::to_camel_case(field_ident.as_ref())
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

        let meta_field = quote!{
            {
                let field = registry.field::<#field_ty>(#name, &());
                let field = #build_description;
                let field = #build_deprecation;
                field
            },
        };
        meta_fields.push(meta_field);

        // Build from_input clause.

        let resolver = quote!{
            #name => executor.resolve_with_ctx(&(), &self.#field_ident),
        };
        resolvers.push(resolver);
    }

    let toks = quote! {
        impl ::juniper::GraphQLType for #ident {
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
