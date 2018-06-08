use std::str::FromStr;

use syn::{
    self,
    DeriveInput,
    NestedMeta,
    Meta,
    Field,
    Fields,
    Data,
    Ident,
};
use quote::{Tokens, ToTokens};

use util::*;

#[derive(Default, Debug)]
struct ObjAttrs {
    name: Option<String>,
    description: Option<String>,
    internal: bool,
}

impl ObjAttrs {
    fn from_input(input: &DeriveInput) -> ObjAttrs {
        let mut res = ObjAttrs::default();

        // Check doc comments for description.
        res.description = get_doc_comment(&input.attrs);

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
                match item {
                    NestedMeta::Meta(Meta::Word(ref ident)) => {
                        if ident == "_internal" {
                            res.internal = true;
                            continue;
                        }
                    }
                    _ => {}
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
                if let Some(val) = keyed_item_value(&item, "default", true) {
                    res.default_expr = Some(val);
                    continue;
                }
                match item {
                    NestedMeta::Meta(Meta::Word(ref ident)) => {
                        if ident == "default" {
                            res.default = true;
                            continue;
                        }
                    }
                    _ => {}
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

pub fn impl_input_object(ast: &syn::DeriveInput) -> Tokens {
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
    let name = attrs.name.unwrap_or(ast.ident.to_string());

    let meta_description = match attrs.description {
        Some(descr) => quote!{ let meta = meta.description(#descr); },
        None => quote!{ let meta = meta; },
    };

    let mut meta_fields = Vec::<Tokens>::new();
    let mut from_inputs = Vec::<Tokens>::new();
    let mut to_inputs = Vec::<Tokens>::new();

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
        let field_description = match field_attrs.description {
            Some(s) => quote!{ let field = field.description(#s); },
            None => quote!{},
        };

        let default = {
            if field_attrs.default {
                Some(quote! { Default::default() })
            } else {
                match field_attrs.default_expr {
                    Some(ref def) => match ::proc_macro::TokenStream::from_str(def) {
                        Ok(t) => {
                            match syn::parse::<syn::Expr>(t) {
                                Ok(e) => {
                                    Some(e.into_tokens())
                                },
                                Err(_) => {
                                    panic!("#graphql(default = ?) must be a valid Rust expression inside a string");
                                },
                            }
                        },
                        Err(_) => {
                            panic!("#graphql(default = ?) must be a valid Rust expression inside a string");
                        }
                    },
                    None => None,
                }
            }
        };

        let create_meta_field = match default {
            Some(ref def) => {
                quote!{
                    let field = registry.arg_with_default::<#field_ty>( #name, &#def, &());
                }
            }
            None => {
                quote!{
                    let field = registry.arg::<#field_ty>(#name, &());
                }
            }
        };
        let meta_field = quote!{
            {
                #create_meta_field
                #field_description
                field
            },
        };
        meta_fields.push(meta_field);

        // Build from_input clause.

        let from_input_default = match default {
            Some(ref def) => {
                quote!{
                    Some(&&_juniper::InputValue::Null) | None if true => #def,
                }
            }
            None => quote!{},
        };

        let from_input = quote!{
            #field_ident: {
                // TODO: investigate the unwraps here, they seem dangerous!
                match obj.get(#name) {
                    #from_input_default
                    Some(v) => _juniper::FromInputValue::from_input_value(v).unwrap(),
                    _ => {
                        _juniper::FromInputValue::from_input_value(&_juniper::InputValue::null())
                            .unwrap()
                    },
                }
            },
        };
        from_inputs.push(from_input);

        // Build to_input clause.
        let to_input = quote!{
            (#name, self.#field_ident.to_input_value()),
        };
        to_inputs.push(to_input);
    }

    let body = quote! {
        impl _juniper::GraphQLType for #ident {
            type Context = ();
            type TypeInfo = ();

            fn name(_: &()) -> Option<&'static str> {
                Some(#name)
            }

            fn meta<'r>(
                _: &(),
                registry: &mut _juniper::Registry<'r>
            ) -> _juniper::meta::MetaType<'r> {
                let fields = &[
                    #(#meta_fields)*
                ];
                let meta = registry.build_input_object_type::<#ident>(&(), fields);
                #meta_description
                meta.into_meta()
            }
        }

        impl _juniper::FromInputValue for #ident {
            fn from_input_value(value: &_juniper::InputValue) -> Option<#ident> {
                if let Some(obj) = value.to_object_value() {
                    let item = #ident {
                        #(#from_inputs)*
                    };
                    Some(item)
                }
                else {
                    None
                }
            }
        }

        impl _juniper::ToInputValue for #ident {
            fn to_input_value(&self) -> _juniper::InputValue {
                _juniper::InputValue::object(vec![
                    #(#to_inputs)*
                ].into_iter().collect())
            }
        }
    };

    let dummy_const = Ident::from(format!("_IMPL_GRAPHQLINPUTOBJECT_FOR_{}", ident).as_str());

    // This ugly hack makes it possible to use the derive inside juniper itself.
    // FIXME: Figure out a better way to do this!
    let crate_reference = if attrs.internal {
        quote! {
            #[doc(hidden)]
            mod _juniper {
                pub use ::{
                    InputValue,
                    FromInputValue,
                    GraphQLType,
                    Registry,
                    meta,
                    ToInputValue
                };
            }
        }
    } else {
        quote! {
            extern crate juniper as _juniper;
        }
    };
    let generated = quote! {
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        #[doc(hidden)]
        const #dummy_const : () = {
            #crate_reference
            #body
        };
    };

    generated
}
