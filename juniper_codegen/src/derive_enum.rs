use syn;
use syn::{
    DeriveInput,
    Meta,
    NestedMeta,
    Data,
    Fields,
    Ident,
    Variant,
};
use quote::Tokens;

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
                if let Some(val) = keyed_item_value(&item, "deprecated", true) {
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

pub fn impl_enum(ast: &syn::DeriveInput) -> Tokens {
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

    let mut values = Vec::<Tokens>::new();
    let mut resolves = Vec::<Tokens>::new();
    let mut from_inputs = Vec::<Tokens>::new();
    let mut to_inputs = Vec::<Tokens>::new();

    for variant in variants {
        match variant.fields {
            Fields::Unit => {},
            _ => {
                panic!(format!(
                    "Invalid enum variant {}.\nGraphQL enums may only contain unit variants.",
                    variant.ident
                ));
            }
        } ;

        let var_attrs = EnumVariantAttrs::from_input(variant);
        let var_ident = &variant.ident;

        // Build value.
        let name = var_attrs
            .name
            .unwrap_or(::util::to_upper_snake_case(variant.ident.as_ref()));
        let descr = match var_attrs.description {
            Some(s) => quote!{ Some(#s.to_string())  },
            None => quote!{ None },
        };
        let depr = match var_attrs.deprecation {
            Some(s) => quote!{ Some(#s.to_string())  },
            None => quote!{ None },
        };
        let value = quote!{
            _juniper::meta::EnumValue{
                name: #name.to_string(),
                description: #descr,
                deprecation_reason: #depr,
            },
        };
        values.push(value);

        // Build resolve match clause.
        let resolve = quote!{
            &#ident::#var_ident => _juniper::Value::String(#name.to_string()),
        };
        resolves.push(resolve);

        // Buil from_input clause.
        let from_input = quote!{
            Some(#name) => Some(#ident::#var_ident),
        };
        from_inputs.push(from_input);

        // Buil to_input clause.
        let to_input = quote!{
            &#ident::#var_ident =>
                _juniper::InputValue::string(#name.to_string()),
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

    let dummy_const = Ident::from(format!("_IMPL_GRAPHQLENUM_FOR_{}", ident).as_str());

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
