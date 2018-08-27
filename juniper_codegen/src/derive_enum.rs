use proc_macro2::{Span, TokenStream};

use syn;
use syn::{Data, DeriveInput, Fields, Ident, Meta, NestedMeta, Variant};

use util::*;

#[derive(Default, Debug)]
struct EnumAttrs {
    name: Option<String>,
    description: Option<String>,
    internal: bool,
}

impl EnumAttrs {
    fn from_input(input: &DeriveInput) -> EnumAttrs {
        let mut res = EnumAttrs {
            name: None,
            description: None,
            /// Flag to specify whether the calling crate is the "juniper" crate itself.
            internal: false,
        };

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
                    "Unknown attribute for #[derive(GraphQLEnum)]: {:?}",
                    item
                ));
            }
        }
        res
    }
}

#[derive(Default)]
struct EnumVariantAttrs {
    name: Option<String>,
    description: Option<String>,
    deprecation: Option<String>,
}

impl EnumVariantAttrs {
    fn from_input(variant: &Variant) -> EnumVariantAttrs {
        let mut res = EnumVariantAttrs::default();

        // Check doc comments for description.
        res.description = get_doc_comment(&variant.attrs);

        // Check attributes for name and description.
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
                if let Some(AttributeValue::String(val)) = keyed_item_value(&item, "deprecated", AttributeValidation::String)  {
                    res.deprecation = Some(val);
                    continue;
                }
                panic!(format!(
                    "Unknown attribute for #[derive(GraphQLEnum)]: {:?}",
                    item
                ));
            }
        }
        res
    }
}

pub fn impl_enum(ast: &syn::DeriveInput) -> TokenStream {
    let variants = match ast.data {
        Data::Enum(ref enum_data) => enum_data.variants.iter().collect::<Vec<_>>(),
        _ => {
            panic!("#[derive(GraphlQLEnum)] may only be applied to enums, not to structs");
        }
    };

    // Parse attributes.
    let ident = &ast.ident;
    let attrs = EnumAttrs::from_input(ast);
    let name = attrs.name.unwrap_or(ast.ident.to_string());

    let meta_description = match attrs.description {
        Some(descr) => quote!{ let meta = meta.description(#descr); },
        None => quote!{ let meta = meta; },
    };

    let mut values = TokenStream::new();
    let mut resolves = TokenStream::new();
    let mut from_inputs = TokenStream::new();
    let mut to_inputs = TokenStream::new();

    for variant in variants {
        match variant.fields {
            Fields::Unit => {}
            _ => {
                panic!(format!(
                    "Invalid enum variant {}.\nGraphQL enums may only contain unit variants.",
                    variant.ident
                ));
            }
        };

        let var_attrs = EnumVariantAttrs::from_input(variant);
        let var_ident = &variant.ident;

        // Build value.
        let name = var_attrs
            .name
            .unwrap_or(::util::to_upper_snake_case(&variant.ident.to_string()));
        let descr = match var_attrs.description {
            Some(s) => quote!{ Some(#s.to_string())  },
            None => quote!{ None },
        };
        let depr = match var_attrs.deprecation {
            Some(s) => quote!{ Some(#s.to_string())  },
            None => quote!{ None },
        };
        values.extend(quote!{
            _juniper::meta::EnumValue{
                name: #name.to_string(),
                description: #descr,
                deprecation_reason: #depr,
            },
        });

        // Build resolve match clause.
        resolves.extend(quote!{
            &#ident::#var_ident => _juniper::Value::String(#name.to_string()),
        });

        // Build from_input clause.
        from_inputs.extend(quote!{
            Some(#name) => Some(#ident::#var_ident),
        });

        // Build to_input clause.
        to_inputs.extend(quote!{
            &#ident::#var_ident =>
                _juniper::InputValue::string(#name.to_string()),
        });
    }

    let body = quote! {
        impl _juniper::GraphQLType for #ident {
            type Context = ();
            type TypeInfo = ();

            fn name(_: &()) -> Option<&'static str> {
                Some(#name)
            }

            fn meta<'r>(_: &(), registry: &mut _juniper::Registry<'r>)
                -> _juniper::meta::MetaType<'r>
            {
                let meta = registry.build_enum_type::<#ident>(&(), &[
                    #(#values)*
                ]);
                #meta_description
                meta.into_meta()
            }

            fn resolve(
                &self,
                _: &(),
                _: Option<&[_juniper::Selection]>,
                _: &_juniper::Executor<Self::Context>
            ) -> _juniper::Value {
                match self {
                    #(#resolves)*
                }
            }
        }

        impl _juniper::FromInputValue for #ident {
            fn from_input_value(v: &_juniper::InputValue) -> Option<#ident> {
                match v.as_enum_value().or_else(|| v.as_string_value()) {
                    #(#from_inputs)*
                    _ => None,
                }
            }
        }

        impl _juniper::ToInputValue for #ident {
            fn to_input_value(&self) -> _juniper::InputValue {
                match self {
                    #(#to_inputs)*
                }
            }
        }
    };

    let dummy_const = Ident::new(
        &format!("_IMPL_GRAPHQLENUM_FOR_{}", ident),
        Span::call_site(),
    );

    // This ugly hack makes it possible to use the derive inside juniper itself.
    // FIXME: Figure out a better way to do this!
    let crate_reference = if attrs.internal {
        quote! {
            #[doc(hidden)]
            mod _juniper {
                pub use ::{
                    InputValue,
                    Value,
                    ToInputValue,
                    FromInputValue,
                    Executor,
                    Selection,
                    Registry,
                    GraphQLType,
                    meta
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
