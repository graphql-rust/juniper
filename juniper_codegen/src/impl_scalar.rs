#![allow(clippy::collapsible_if)]

use crate::util;
use proc_macro::TokenStream;
use quote::quote;

#[derive(Debug)]
struct ScalarCodegenInput {
    impl_for_type: Option<syn::PathSegment>,
    custom_data_type: Option<syn::PathSegment>,
    custom_data_type_is_struct: bool,
    resolve_body: Option<syn::Block>,
    from_input_value_arg: Option<syn::Ident>,
    from_input_value_body: Option<syn::Block>,
    from_input_value_result: Option<syn::Type>,
    from_str_arg: Option<syn::Ident>,
    from_str_body: Option<syn::Block>,
    from_str_result: Option<syn::Type>,
}

fn get_first_method_arg(
    inputs: syn::punctuated::Punctuated<syn::FnArg, syn::Token![,]>,
) -> Option<syn::Ident> {
    if let Some(fn_arg) = inputs.first() {
        match fn_arg {
            syn::FnArg::Typed(pat_type) => match &*pat_type.pat {
                syn::Pat::Ident(pat_ident) => return Some(pat_ident.ident.clone()),
                _ => (),
            },
            _ => (),
        }
    }

    None
}

fn get_method_return_type(output: syn::ReturnType) -> Option<syn::Type> {
    match output {
        syn::ReturnType::Type(_, return_type) => Some(*return_type),
        _ => None,
    }
}

// Find the enum type by inspecting the type parameter on the return value
fn get_enum_type(return_type: &Option<syn::Type>) -> Option<syn::PathSegment> {
    if let Some(return_type) = return_type {
        match return_type {
            syn::Type::Path(type_path) => {
                let path_segment = type_path
                    .path
                    .segments
                    .iter()
                    .find(|ps| match ps.arguments {
                        syn::PathArguments::AngleBracketed(_) => true,
                        _ => false,
                    });

                if let Some(path_segment) = path_segment {
                    match &path_segment.arguments {
                        syn::PathArguments::AngleBracketed(generic_args) => {
                            let generic_type_arg =
                                generic_args.args.iter().find(|generic_type_arg| {
                                    match generic_type_arg {
                                        syn::GenericArgument::Type(_) => true,
                                        _ => false,
                                    }
                                });

                            if let Some(generic_type_arg) = generic_type_arg {
                                match generic_type_arg {
                                    syn::GenericArgument::Type(the_type) => match the_type {
                                        syn::Type::Path(type_path) => {
                                            if let Some(path_segment) =
                                                type_path.path.segments.first()
                                            {
                                                return Some(path_segment.clone());
                                            }
                                        }
                                        _ => (),
                                    },
                                    _ => (),
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }
    }

    None
}

impl syn::parse::Parse for ScalarCodegenInput {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let mut impl_for_type: Option<syn::PathSegment> = None;
        let mut enum_data_type: Option<syn::PathSegment> = None;
        let mut resolve_body: Option<syn::Block> = None;
        let mut from_input_value_arg: Option<syn::Ident> = None;
        let mut from_input_value_body: Option<syn::Block> = None;
        let mut from_input_value_result: Option<syn::Type> = None;
        let mut from_str_arg: Option<syn::Ident> = None;
        let mut from_str_body: Option<syn::Block> = None;
        let mut from_str_result: Option<syn::Type> = None;

        let parse_custom_scalar_value_impl: syn::ItemImpl = input.parse()?;
        // To implement a custom scalar for a struct, it's required to
        // specify a generic type and a type bound
        let custom_data_type_is_struct: bool =
            !parse_custom_scalar_value_impl.generics.params.is_empty();

        match *parse_custom_scalar_value_impl.self_ty {
            syn::Type::Path(type_path) => {
                if let Some(path_segment) = type_path.path.segments.first() {
                    impl_for_type = Some(path_segment.clone());
                }
            }
            _ => (),
        }

        for impl_item in parse_custom_scalar_value_impl.items {
            match impl_item {
                syn::ImplItem::Method(method) => match method.sig.ident.to_string().as_str() {
                    "resolve" => {
                        resolve_body = Some(method.block);
                    }
                    "from_input_value" => {
                        from_input_value_arg = get_first_method_arg(method.sig.inputs);
                        from_input_value_result = get_method_return_type(method.sig.output);
                        from_input_value_body = Some(method.block);
                    }
                    "from_str" => {
                        from_str_arg = get_first_method_arg(method.sig.inputs);
                        from_str_result = get_method_return_type(method.sig.output);

                        if !custom_data_type_is_struct {
                            enum_data_type = get_enum_type(&from_str_result);
                        }

                        from_str_body = Some(method.block);
                    }
                    _ => (),
                },
                _ => (),
            };
        }

        let custom_data_type = if custom_data_type_is_struct {
            impl_for_type.clone()
        } else {
            enum_data_type
        };

        Ok(ScalarCodegenInput {
            impl_for_type,
            custom_data_type,
            custom_data_type_is_struct,
            resolve_body,
            from_input_value_arg,
            from_input_value_body,
            from_input_value_result,
            from_str_arg,
            from_str_body,
            from_str_result,
        })
    }
}

/// Generate code for the juniper::graphql_scalar proc macro.
pub fn build_scalar(attributes: TokenStream, body: TokenStream, is_internal: bool) -> TokenStream {
    let attrs = match syn::parse::<util::FieldAttributes>(attributes) {
        Ok(attrs) => attrs,
        Err(e) => {
            panic!("Invalid attributes:\n{}", e);
        }
    };

    let input = syn::parse_macro_input!(body as ScalarCodegenInput);

    let impl_for_type = input
        .impl_for_type
        .expect("Unable to find target for implementation target for `GraphQLScalar`");
    let custom_data_type = input
        .custom_data_type
        .expect("Unable to find custom scalar data type");
    let resolve_body = input
        .resolve_body
        .expect("Unable to find body of `resolve` method");
    let from_input_value_arg = input
        .from_input_value_arg
        .expect("Unable to find argument for `from_input_value` method");
    let from_input_value_body = input
        .from_input_value_body
        .expect("Unable to find body of `from_input_value` method");
    let from_input_value_result = input
        .from_input_value_result
        .expect("Unable to find return type of `from_input_value` method");
    let from_str_arg = input
        .from_str_arg
        .expect("Unable to find argument for `from_str` method");
    let from_str_body = input
        .from_str_body
        .expect("Unable to find body of `from_str` method");
    let from_str_result = input
        .from_str_result
        .expect("Unable to find return type of `from_str` method");

    let name = attrs
        .name
        .unwrap_or_else(|| impl_for_type.ident.to_string());
    let crate_name = match is_internal {
        true => quote!(crate),
        _ => quote!(juniper),
    };
    let description = match attrs.description {
        Some(val) => quote!(.description(#val)),
        None => quote!(),
    };
    let async_generic_type = match input.custom_data_type_is_struct {
        true => quote!(__S),
        _ => quote!(#custom_data_type),
    };
    let async_generic_type_decl = match input.custom_data_type_is_struct {
        true => quote!(<#async_generic_type>),
        _ => quote!(),
    };
    let generic_type = match input.custom_data_type_is_struct {
        true => quote!(S),
        _ => quote!(#custom_data_type),
    };
    let generic_type_decl = match input.custom_data_type_is_struct {
        true => quote!(<#generic_type>),
        _ => quote!(),
    };
    let generic_type_bound = match input.custom_data_type_is_struct {
        true => quote!(where #generic_type: #crate_name::ScalarValue,),
        _ => quote!(),
    };

    let _async = quote!(
        impl#async_generic_type_decl #crate_name::GraphQLTypeAsync<#async_generic_type> for #impl_for_type
        where
            #async_generic_type: #crate_name::ScalarValue + Send + Sync,
            Self: #crate_name::GraphQLType<#async_generic_type> + Send + Sync,
            Self::Context: Send + Sync,
            Self::TypeInfo: Send + Sync,
        {
            fn resolve_async<'a>(
                &'a self,
                info: &'a Self::TypeInfo,
                selection_set: Option<&'a [#crate_name::Selection<#async_generic_type>]>,
                executor: &'a #crate_name::Executor<Self::Context, #async_generic_type>,
            ) -> #crate_name::BoxFuture<'a, #crate_name::ExecutionResult<#async_generic_type>> {
                use #crate_name::GraphQLType;
                use futures::future;
                let v = self.resolve(info, selection_set, executor);
                Box::pin(future::ready(v))
            }
        }
    );

    quote!(
        #_async

        impl#generic_type_decl #crate_name::GraphQLType<#generic_type> for #impl_for_type
        #generic_type_bound
        {
            type Context = ();
            type TypeInfo = ();

            fn name(_: &Self::TypeInfo) -> Option<&str> {
                Some(#name)
            }

            fn meta<'r>(
                info: &Self::TypeInfo,
                registry: &mut #crate_name::Registry<'r, #generic_type>,
            ) -> #crate_name::meta::MetaType<'r, #generic_type>
            where
                #generic_type: 'r,
            {
                registry.build_scalar_type::<Self>(info)
                    #description
                    .into_meta()
            }

            fn resolve(
                &self,
                info: &(),
                selection: Option<&[#crate_name::Selection<#generic_type>]>,
                executor: &#crate_name::Executor<Self::Context, #generic_type>,
            ) -> #crate_name::ExecutionResult<#generic_type> {
                Ok(#resolve_body)
            }
        }

        impl#generic_type_decl #crate_name::ToInputValue<#generic_type> for #impl_for_type
        #generic_type_bound
        {
            fn to_input_value(&self) -> #crate_name::InputValue<#generic_type> {
                let v = #resolve_body;
                #crate_name::ToInputValue::to_input_value(&v)
            }
        }

        impl#generic_type_decl #crate_name::FromInputValue<#generic_type> for #impl_for_type
        #generic_type_bound
        {
            fn from_input_value(#from_input_value_arg: &#crate_name::InputValue<#generic_type>) -> #from_input_value_result {
                #from_input_value_body
            }
        }

        impl#generic_type_decl #crate_name::ParseScalarValue<#generic_type> for #impl_for_type
        #generic_type_bound
            {
                fn from_str<'a>(
                    #from_str_arg: #crate_name::parser::ScalarToken<'a>,
                ) -> #from_str_result {
                #from_str_body
            }
        }
    ).into()
}
