use proc_macro2::{Span, TokenStream};
use syn;
use syn::{Data, DeriveInput, Field, Fields, Ident};

use util::*;

#[derive(Default, Debug)]
struct ObjAttrs {
    name: Option<String>,
    description: Option<String>,
    scalar: Option<Ident>,
}

impl ObjAttrs {
    fn from_input(input: &DeriveInput) -> ObjAttrs {
        let mut res = ObjAttrs::default();

        // Check doc comments for description.
        res.description = get_doc_comment(&input.attrs);

        // Check attributes for name and description.
        if let Some(items) = get_graphql_attr(&input.attrs) {
            for item in items {
                if let Some(AttributeValue::String(val)) =
                    keyed_item_value(&item, "name", AttributeValidation::String)
                {
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
                if let Some(AttributeValue::String(val)) =
                    keyed_item_value(&item, "description", AttributeValidation::String)
                {
                    res.description = Some(val);
                    continue;
                }
                if let Some(AttributeValue::String(scalar)) =
                    keyed_item_value(&item, "scalar", AttributeValidation::String)
                {
                    res.scalar = Some(Ident::new(&scalar as &str, Span::call_site()));
                    continue;
                }
                panic!(format!(
                    "Unknown struct attribute for #[derive(GraphQLObject)]: {:?}",
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
    deprecation: Option<DeprecationAttr>,
    skip: bool,
}

impl ObjFieldAttrs {
    fn from_input(variant: &Field) -> ObjFieldAttrs {
        let mut res = ObjFieldAttrs::default();

        // Check doc comments for description.
        res.description = get_doc_comment(&variant.attrs);

        // Check builtin deprecated attribute for deprecation.
        res.deprecation = get_deprecated(&variant.attrs);

        // Check attributes.
        if let Some(items) = get_graphql_attr(&variant.attrs) {
            for item in items {
                if let Some(AttributeValue::String(val)) =
                    keyed_item_value(&item, "name", AttributeValidation::String)
                {
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
                if let Some(AttributeValue::String(val)) =
                    keyed_item_value(&item, "description", AttributeValidation::String)
                {
                    res.description = Some(val);
                    continue;
                }
                if let Some(AttributeValue::String(val)) =
                    keyed_item_value(&item, "deprecation", AttributeValidation::String)
                {
                    res.deprecation = Some(DeprecationAttr { reason: Some(val) });
                    continue;
                }
                match keyed_item_value(&item, "deprecated", AttributeValidation::String) {
                    Some(AttributeValue::String(val)) => {
                        res.deprecation = Some(DeprecationAttr { reason: Some(val) });
                        continue;
                    }
                    Some(AttributeValue::Bare) => {
                        res.deprecation = Some(DeprecationAttr { reason: None });
                        continue;
                    }
                    None => {}
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
        Some(s) => quote! { builder.description(#s)  },
        None => quote! { builder },
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
            Some(s) => quote! { field.description(#s)  },
            None => quote! { field },
        };

        let build_deprecation = match field_attrs.deprecation {
            Some(DeprecationAttr { reason: Some(s) }) => quote! { field.deprecated(Some(#s)) },
            Some(DeprecationAttr { reason: None }) => quote! { field.deprecated(None) },
            None => quote! { field },
        };

        meta_fields.extend(quote! {
            {
                let field = registry.field::<#field_ty>(#name, &());
                let field = #build_description;
                let field = #build_deprecation;
                field
            },
        });

        // Build from_input clause.

        resolvers.extend(quote! {
            #name => executor.resolve_with_ctx(&(), &self.#field_ident),
        });
    }

    let (_, ty_generics, _) = generics.split_for_impl();

    let mut generics = generics.clone();

    if attrs.scalar.is_none() {
        generics.params.push(parse_quote!(__S));
        {
            let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
            where_clause
                .predicates
                .push(parse_quote!(__S: juniper::ScalarValue));
            where_clause
                .predicates
                .push(parse_quote!(for<'__b> &'__b __S: juniper::ScalarRefValue<'__b>));
        }
    }

    let scalar = attrs
        .scalar
        .unwrap_or_else(|| Ident::new("__S", Span::call_site()));

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let body = quote! {
        impl#impl_generics juniper::GraphQLType<#scalar> for #ident #ty_generics
            #where_clause
        {
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
                registry: &mut juniper::Registry<'r, #scalar>
            ) -> juniper::meta::MetaType<'r, #scalar>
                where #scalar: 'r
            {
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
                _: &juniper::Arguments<#scalar>,
                executor: &juniper::Executor<Self::Context, #scalar>
            ) -> juniper::ExecutionResult<#scalar>
            {

                match field_name {
                    #(#resolvers)*
                    _ => panic!("Field {} not found on type {}", field_name, #ident_name),
                }

            }
        }
    };
    body
}
