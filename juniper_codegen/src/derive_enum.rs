use syn;
use syn::*;
use quote::Tokens;

use util::*;


#[derive(Default, Debug)]
struct EnumAttrs {
    name: Option<String>,
    description: Option<String>,
}

impl EnumAttrs {
    fn from_input(input: &DeriveInput) -> EnumAttrs {
        let mut res = EnumAttrs::default();

        // Check attributes for name and description.
        if let Some(items) = get_graphl_attr(&input.attrs) {
            for item in items {
                if let Some(val) = keyed_item_value(item, "name", true) {
                    res.name = Some(val);
                    continue;
                }
                if let Some(val) = keyed_item_value(item, "description", true) {
                    res.description = Some(val);
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
                if let Some(val) = keyed_item_value(item, "name", true) {
                    res.name = Some(val);
                    continue;
                }
                if let Some(val) = keyed_item_value(item, "description", true) {
                    res.description = Some(val);
                    continue;
                }
                if let Some(val) = keyed_item_value(item, "deprecated", true) {
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
    let variants = match ast.body {
        Body::Enum(ref var) => var,
        Body::Struct(_) => {
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
        if variant.data != VariantData::Unit {
            panic!(format!(
                "Invalid enum variant {}.\nGraphQL enums may only contain unit variants.",
                variant.ident
            ));
        }
        let var_attrs = EnumVariantAttrs::from_input(variant);
        let var_ident = &variant.ident;

        // Build value.
        let name = var_attrs
            .name
            .unwrap_or(variant.ident.as_ref().to_uppercase());
        let descr = match var_attrs.description {
            Some(s) => quote!{ Some(#s.to_string())  },
            None => quote!{ None },
        };
        let depr = match var_attrs.deprecation {
            Some(s) => quote!{ Some(#s.to_string())  },
            None => quote!{ None },
        };
        let value = quote!{
            ::juniper::meta::EnumValue{
                name: #name.to_string(),
                description: #descr,
                deprecation_reason: #depr,
            },
        };
        values.push(value);

        // Build resolve match clause.
        let resolve = quote!{
            &#ident::#var_ident => ::juniper::Value::String(#name.to_string()),
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
                ::juniper::InputValue::string(#name.to_string()),
        };
        to_inputs.push(to_input);
    }

    quote! {
        impl ::juniper::GraphQLType for #ident {
            type Context = ();

            fn name() -> Option<&'static str> {
                Some(#name)
            }

            fn meta<'r>(registry: &mut ::juniper::Registry<'r>) -> ::juniper::meta::MetaType<'r> {
                let meta = registry.build_enum_type::<#ident>(&[
                    #(#values)*
                ]);
                #meta_description
                meta.into_meta()
            }

            fn resolve(&self, _: Option<&[::juniper::Selection]>, _: &::juniper::Executor<Self::Context>) -> ::juniper::Value {
                match self {
                    #(#resolves)*
                }
            }
        }

        impl ::juniper::FromInputValue for #ident {
            fn from(v: &::juniper::InputValue) -> Option<#ident> {
                match v.as_enum_value().or_else(|| v.as_string_value()) {
                    #(#from_inputs)*
                    _ => None,
                }
            }
        }

        impl ::juniper::ToInputValue for #ident {
            fn to(&self) -> ::juniper::InputValue {
                match self {
                    #(#to_inputs)*
                }
            }
        }
    }
}
