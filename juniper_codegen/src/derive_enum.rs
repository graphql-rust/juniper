use proc_macro2::TokenStream;

use quote::quote;
use syn::{self, Data, DeriveInput, Fields, Variant};

use crate::util::*;

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

pub fn impl_enum(ast: &syn::DeriveInput, is_internal: bool) -> TokenStream {
    let juniper_path = if is_internal {
        quote!(crate)
    } else {
        quote!(juniper)
    };

    let variants = match ast.data {
        Data::Enum(ref enum_data) => enum_data.variants.iter().collect::<Vec<_>>(),
        _ => {
            panic!("#[derive(GraphlQLEnum)] may only be applied to enums, not to structs");
        }
    };

    // Parse attributes.
    let ident = &ast.ident;
    let attrs = EnumAttrs::from_input(ast);
    let name = attrs.name.unwrap_or_else(|| ast.ident.to_string());

    let meta_description = match attrs.description {
        Some(descr) => quote! { let meta = meta.description(#descr); },
        None => quote! { let meta = meta; },
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
            .unwrap_or_else(|| crate::util::to_upper_snake_case(&variant.ident.to_string()));
        let descr = match var_attrs.description {
            Some(s) => quote! { Some(#s.to_string())  },
            None => quote! { None },
        };
        let depr = match var_attrs.deprecation {
            Some(DeprecationAttr { reason: Some(s) }) => quote! {
                #juniper_path::meta::DeprecationStatus::Deprecated(Some(#s.to_string()))
            },
            Some(DeprecationAttr { reason: None }) => quote! {
                #juniper_path::meta::DeprecationStatus::Deprecated(None)
            },
            None => quote! {
                #juniper_path::meta::DeprecationStatus::Current
            },
        };
        values.extend(quote! {
            #juniper_path::meta::EnumValue{
                name: #name.to_string(),
                description: #descr,
                deprecation_status: #depr,
            },
        });

        // Build resolve match clause.
        resolves.extend(quote! {
            &#ident::#var_ident => #juniper_path::Value::scalar(String::from(#name)),
        });

        // Build from_input clause.
        from_inputs.extend(quote! {
            Some(#name) => Some(#ident::#var_ident),
        });

        // Build to_input clause.
        to_inputs.extend(quote! {
            &#ident::#var_ident =>
                #juniper_path::InputValue::scalar(#name.to_string()),
        });
    }

    let _async = quote!(
        impl<__S> #juniper_path::GraphQLTypeAsync<__S> for #ident
            where
                __S: #juniper_path::ScalarValue + Send + Sync,
        {
            fn resolve_async<'a>(
                &'a self,
                info: &'a Self::TypeInfo,
                selection_set: Option<&'a [#juniper_path::Selection<__S>]>,
                executor: &'a #juniper_path::Executor<Self::Context, __S>,
            ) -> #juniper_path::BoxFuture<'a, #juniper_path::ExecutionResult<__S>> {
                use #juniper_path::GraphQLType;
                use futures::future;
                let v = self.resolve(info, selection_set, executor);
                future::FutureExt::boxed(future::ready(v))
            }
        }
    );

    let body = quote! {
        impl<__S> #juniper_path::GraphQLType<__S> for #ident
        where __S:
            #juniper_path::ScalarValue,
        {
            type Context = ();
            type TypeInfo = ();

            fn name(_: &()) -> Option<&'static str> {
                Some(#name)
            }

            fn meta<'r>(_: &(), registry: &mut #juniper_path::Registry<'r, __S>)
                        -> #juniper_path::meta::MetaType<'r, __S>
            where __S: 'r,
            {
                let meta = registry.build_enum_type::<#ident>(&(), &[
                    #values
                ]);
                #meta_description
                meta.into_meta()
            }

            fn resolve(
                &self,
                _: &(),
                _: Option<&[#juniper_path::Selection<__S>]>,
                _: &#juniper_path::Executor<Self::Context, __S>
            ) -> #juniper_path::ExecutionResult<__S> {
                let v = match self {
                    #resolves
                };
                Ok(v)
            }
        }

        impl<__S: #juniper_path::ScalarValue> #juniper_path::FromInputValue<__S> for #ident {
            fn from_input_value(v: &#juniper_path::InputValue<__S>) -> Option<#ident>
            {
                match v.as_enum_value().or_else(|| {
                    v.as_string_value()
                }) {
                    #from_inputs
                    _ => None,
                }
            }
        }

        impl<__S: #juniper_path::ScalarValue> #juniper_path::ToInputValue<__S> for #ident {
            fn to_input_value(&self) -> #juniper_path::InputValue<__S> {
                match self {
                    #to_inputs
                }
            }
        }

        #_async
    };

    body
}
