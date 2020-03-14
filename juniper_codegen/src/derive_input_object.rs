use std::str::FromStr;

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{self, parse_quote, Data, DeriveInput, Field, Fields, Ident, Meta, NestedMeta};

use crate::util::*;

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
                    "Unknown attribute for #[derive(GraphQLInputObject)]: {:?}",
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
    default: bool,
    default_expr: Option<String>,
}

impl ObjFieldAttrs {
    fn from_input(variant: &Field) -> ObjFieldAttrs {
        let mut res = ObjFieldAttrs::default();

        // Check doc comments for description.
        res.description = get_doc_comment(&variant.attrs);

        // Check attributes for name and description.
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
                    keyed_item_value(&item, "default", AttributeValidation::Any)
                {
                    res.default_expr = Some(val);
                    continue;
                }

                if let NestedMeta::Meta(Meta::Path(ref path)) = item {
                    if path.is_ident("default") {
                        res.default = true;
                        continue;
                    }
                }
                panic!(format!(
                    "Unknown attribute for #[derive(GraphQLInputObject)]: {:?}",
                    item
                ));
            }
        }
        res
    }
}

pub fn impl_input_object(ast: &syn::DeriveInput, is_internal: bool) -> TokenStream {
    let juniper_path = if is_internal {
        quote!(crate)
    } else {
        quote!(juniper)
    };

    let fields = match ast.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref named) => named.named.iter().collect::<Vec<_>>(),
            _ => {
                panic!(
                    "#[derive(GraphQLInputObject)] may only be used on regular structs with fields"
                );
            }
        },
        _ => {
            panic!("#[derive(GraphlQLInputObject)] may only be applied to structs, not to enums");
        }
    };

    // Parse attributes.
    let ident = &ast.ident;
    let attrs = ObjAttrs::from_input(ast);
    let name = attrs.name.unwrap_or_else(|| ast.ident.to_string());
    let generics = &ast.generics;

    let meta_description = match attrs.description {
        Some(descr) => quote! { let meta = meta.description(#descr); },
        None => quote! { let meta = meta; },
    };

    let mut meta_fields = TokenStream::new();
    let mut from_inputs = TokenStream::new();
    let mut to_inputs = TokenStream::new();

    let (_, ty_generics, _) = generics.split_for_impl();

    let mut generics = generics.clone();

    let scalar = if let Some(scalar) = attrs.scalar {
        scalar
    } else {
        generics.params.push(parse_quote!(__S));
        {
            let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
            where_clause
                .predicates
                .push(parse_quote!(__S: #juniper_path::ScalarValue));
        }
        Ident::new("__S", Span::call_site())
    };

    let (impl_generics, _, where_clause) = generics.split_for_impl();

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
                crate::util::to_camel_case(&unraw(&field_ident.to_string()))
            }
        };
        let field_description = match field_attrs.description {
            Some(s) => quote! { let field = field.description(#s); },
            None => quote! {},
        };

        let default = {
            if field_attrs.default {
                Some(quote! { Default::default() })
            } else {
                match field_attrs.default_expr {
                    Some(ref def) => match proc_macro::TokenStream::from_str(def) {
                        Ok(t) => match syn::parse::<syn::Expr>(t) {
                            Ok(e) => {
                                let mut tokens = TokenStream::new();
                                e.to_tokens(&mut tokens);
                                Some(tokens)
                            }
                            Err(e) => {
                                let _ = e;
                                panic!("#graphql(default = ?) must be a valid Rust expression inside a string");
                            }
                        },
                        Err(e) => {
                            let _ = e;
                            panic!("#graphql(default = ?) must be a valid Rust expression inside a string");
                        }
                    },
                    None => None,
                }
            }
        };

        let create_meta_field = match default {
            Some(ref def) => {
                quote! {
                    let field = registry.arg_with_default::<#field_ty>( #name, &#def, &());
                }
            }
            None => {
                quote! {
                    let field = registry.arg::<#field_ty>(#name, &());
                }
            }
        };
        meta_fields.extend(quote! {
            {
                #create_meta_field
                #field_description
                field
            },
        });

        // Build from_input clause.

        let from_input_default = match default {
            Some(ref def) => {
                quote! {
                    Some(&&#juniper_path::InputValue::Null) | None if true => #def,
                }
            }
            None => quote! {},
        };

        from_inputs.extend(quote!{
            #field_ident: {
                // TODO: investigate the unwraps here, they seem dangerous!
                match obj.get(#name) {
                    #from_input_default
                    Some(ref v) => #juniper_path::FromInputValue::from_input_value(v).unwrap(),
                    None => {
                        #juniper_path::FromInputValue::from_input_value(&#juniper_path::InputValue::<#scalar>::null())
                            .unwrap()
                    },
                }
            },
        });

        // Build to_input clause.
        to_inputs.extend(quote! {
            (#name, self.#field_ident.to_input_value()),
        });
    }

    let body = quote! {
        impl#impl_generics #juniper_path::GraphQLType<#scalar> for #ident #ty_generics
        #where_clause
        {
            type Context = ();
            type TypeInfo = ();

            fn name(_: &()) -> Option<&'static str> {
                Some(#name)
            }

            fn meta<'r>(
                _: &(),
                registry: &mut #juniper_path::Registry<'r, #scalar>
            ) -> #juniper_path::meta::MetaType<'r, #scalar>
                where #scalar: 'r
            {
                let fields = &[
                    #meta_fields
                ];
                let meta = registry.build_input_object_type::<#ident>(&(), fields);
                #meta_description
                meta.into_meta()
            }
        }

        impl#impl_generics #juniper_path::FromInputValue<#scalar> for #ident #ty_generics
        #where_clause
        {
            fn from_input_value(value: &#juniper_path::InputValue<#scalar>) -> Option<Self>
            {
                if let Some(obj) = value.to_object_value() {
                    let item = #ident {
                        #from_inputs
                    };
                    Some(item)
                }
                else {
                    None
                }
            }
        }

        impl#impl_generics #juniper_path::ToInputValue<#scalar> for #ident #ty_generics
        #where_clause
        {
            fn to_input_value(&self) -> #juniper_path::InputValue<#scalar> {
                #juniper_path::InputValue::object(vec![
                    #to_inputs
                ].into_iter().collect())
            }
        }
    };

    body
}
