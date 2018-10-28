use proc_macro2::{Span, TokenStream};

use syn;
use syn::{Data, DeriveInput, Fields, Ident, Variant};

use util::*;

#[derive(Default, Debug)]
struct EnumAttrs {
    name: Option<String>,
    description: Option<String>,
}

impl EnumAttrs {
    fn from_input(input: &DeriveInput) -> EnumAttrs {
        let mut res = EnumAttrs {
            name: None,
            description: None,
        };

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
                panic!(format!(
                    "Unknown enum attribute for #[derive(GraphQLEnum)]: {:?}",
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
    deprecation: Option<DeprecationAttr>,
}

impl EnumVariantAttrs {
    fn from_input(variant: &Variant) -> EnumVariantAttrs {
        let mut res = EnumVariantAttrs::default();

        // Check doc comments for description.
        res.description = get_doc_comment(&variant.attrs);

        // Check builtin deprecated attribute for deprecation.
        res.deprecation = get_deprecated(&variant.attrs);

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
                panic!(format!(
                    "Unknown variant attribute for #[derive(GraphQLEnum)]: {:?}",
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
            Some(DeprecationAttr { reason: Some(s) }) => quote!{
                _juniper::meta::DeprecationStatus::Deprecated(Some(#s.to_string()))
            },
            Some(DeprecationAttr { reason: None }) => quote!{
                _juniper::meta::DeprecationStatus::Deprecated(None)
            },
            None => quote!{
                _juniper::meta::DeprecationStatus::Current
            },
        };
        values.extend(quote!{
            _juniper::meta::EnumValue{
                name: #name.to_string(),
                description: #descr,
                deprecation_status: #depr,
            },
        });

        // Build resolve match clause.
        resolves.extend(quote!{
            &#ident::#var_ident => _juniper::Value::scalar(String::from(#name)),
        });

        // Build from_input clause.
        from_inputs.extend(quote!{
            Some(#name) => Some(#ident::#var_ident),
        });

        // Build to_input clause.
        to_inputs.extend(quote!{
            &#ident::#var_ident =>
                _juniper::InputValue::scalar(#name.to_string()),
        });
    }

    let body = quote! {
        impl<__S> _juniper::GraphQLType<__S> for #ident
        where __S: _juniper::ScalarValue,
            for<'__b> &'__b __S: _juniper::ScalarRefValue<'__b>
        {
            type Context = ();
            type TypeInfo = ();

            fn name(_: &()) -> Option<&'static str> {
                Some(#name)
            }

            fn meta<'r>(_: &(), registry: &mut _juniper::Registry<'r, __S>)
                        -> _juniper::meta::MetaType<'r, __S>
            where __S: 'r,
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
                _: Option<&[_juniper::Selection<__S>]>,
                _: &_juniper::Executor<Self::Context, __S>
            ) -> _juniper::Value<__S> {
                match self {
                    #(#resolves)*
                }
            }
        }

        impl<__S: _juniper::ScalarValue> _juniper::FromInputValue<__S> for #ident {
            fn from_input_value(v: &_juniper::InputValue<__S>) -> Option<#ident>
                where for<'__b> &'__b __S: _juniper::ScalarRefValue<'__b>
            {
                match v.as_enum_value().or_else(|| {
                    v.as_scalar_value::<String>().map(|s| s as &str)
                }) {
                    #(#from_inputs)*
                    _ => None,
                }
            }
        }

        impl<__S: _juniper::ScalarValue> _juniper::ToInputValue<__S> for #ident {
            fn to_input_value(&self) -> _juniper::InputValue<__S> {
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

    let generated = quote! {
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        #[doc(hidden)]
        const #dummy_const : () = {
            mod _juniper {
                __juniper_use_everything!();
            }
            #body
        };
    };

    generated
}
