#![allow(clippy::collapsible_if)]

use crate::util;
use proc_macro::TokenStream;
use quote::quote;

#[derive(Debug)]
struct ScalarCodegenInput {
    ident: Option<syn::Ident>,
    resolve_body: Option<syn::Block>,
    from_input_value_arg: Option<syn::Ident>,
    from_input_value_body: Option<syn::Block>,
    from_input_value_result: Option<syn::Type>,
    from_str_arg: Option<syn::Ident>,
    from_str_body: Option<syn::Block>,
    from_str_result: Option<syn::Type>,
}

impl syn::parse::Parse for ScalarCodegenInput {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let mut ident: Option<syn::Ident> = None;
        let mut resolve_body: Option<syn::Block> = None;
        let mut from_input_value_arg: Option<syn::Ident> = None;
        let mut from_input_value_body: Option<syn::Block> = None;
        let mut from_input_value_result: Option<syn::Type> = None;
        let mut from_str_arg: Option<syn::Ident> = None;
        let mut from_str_body: Option<syn::Block> = None;
        let mut from_str_result: Option<syn::Type> = None;

        let parse_custom_scalar_value_impl: syn::ItemImpl = input.parse()?;

        match *parse_custom_scalar_value_impl.self_ty {
            syn::Type::Path(type_path) => match type_path.path.segments.first() {
                Some(path_segment) => {
                    ident = Some(path_segment.ident.clone());
                }
                _ => (),
            },
            _ => (),
        }

        for impl_item in parse_custom_scalar_value_impl.items {
            match impl_item {
                syn::ImplItem::Method(method) => match method.sig.ident.to_string().as_str() {
                    "resolve" => {
                        resolve_body = Some(method.block);
                    }
                    "from_input_value" => {
                        match method.sig.inputs.first() {
                            Some(fn_arg) => match fn_arg {
                                syn::FnArg::Typed(pat_type) => match &*pat_type.pat {
                                    syn::Pat::Ident(pat_ident) => {
                                        from_input_value_arg = Some(pat_ident.ident.clone())
                                    }
                                    _ => (),
                                },
                                _ => (),
                            },
                            _ => (),
                        }

                        match method.sig.output {
                            syn::ReturnType::Type(_, return_type) => {
                                from_input_value_result = Some(*return_type);
                            }
                            _ => (),
                        }

                        from_input_value_body = Some(method.block);
                    }
                    "from_str" => {
                        match method.sig.inputs.first() {
                            Some(fn_arg) => match fn_arg {
                                syn::FnArg::Typed(pat_type) => match &*pat_type.pat {
                                    syn::Pat::Ident(pat_ident) => {
                                        from_str_arg = Some(pat_ident.ident.clone())
                                    }
                                    _ => (),
                                },
                                _ => (),
                            },
                            _ => (),
                        }

                        match method.sig.output {
                            syn::ReturnType::Type(_, return_type) => {
                                from_str_result = Some(*return_type);
                            }
                            _ => (),
                        }

                        from_str_body = Some(method.block);
                    }
                    _ => (),
                },
                _ => (),
            };
        }

        Ok(ScalarCodegenInput {
            ident,
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
    let ident = input.ident.unwrap();
    let resolve_body = input.resolve_body.unwrap();
    let from_input_value_arg = input.from_input_value_arg.unwrap();
    let from_input_value_body = input.from_input_value_body.unwrap();
    let from_input_value_result = input.from_input_value_result.unwrap();
    let from_str_arg = input.from_str_arg.unwrap();
    let from_str_body = input.from_str_body.unwrap();
    let from_str_result = input.from_str_result.unwrap();

    // TODO: Code below copied from derive_scalar_value.rs#impl_scalar_struct. REFACTOR!

    let name = attrs.name.unwrap_or_else(|| ident.to_string());

    let crate_name = if is_internal {
        quote!(crate)
    } else {
        quote!(juniper)
    };

    let description = match attrs.description {
        Some(val) => quote!( .description( #val ) ),
        None => quote!(),
    };

    let _async = quote!(
        impl<__S> ::#crate_name::GraphQLTypeAsync<__S> for #ident
        where
            __S: #crate_name::ScalarValue + Send + Sync,
            Self: #crate_name::GraphQLType<__S> + Send + Sync,
            Self::Context: Send + Sync,
            Self::TypeInfo: Send + Sync,
        {
            fn resolve_async<'a>(
                &'a self,
                info: &'a Self::TypeInfo,
                selection_set: Option<&'a [#crate_name::Selection<__S>]>,
                executor: &'a #crate_name::Executor<Self::Context, __S>,
            ) -> #crate_name::BoxFuture<'a, #crate_name::ExecutionResult<__S>> {
                use #crate_name::GraphQLType;
                use futures::future;
                let v = self.resolve(info, selection_set, executor);
                Box::pin(future::ready(v))
            }
        }
    );

    quote!(
        #_async

        impl<S> #crate_name::GraphQLType<S> for #ident
        where
            S: #crate_name::ScalarValue,
        {
            type Context = ();
            type TypeInfo = ();

            fn name(_: &Self::TypeInfo) -> Option<&str> {
                Some(#name)
            }

            fn meta<'r>(
                info: &Self::TypeInfo,
                registry: &mut #crate_name::Registry<'r, S>,
            ) -> #crate_name::meta::MetaType<'r, S>
            where
                S: 'r,
            {
                registry.build_scalar_type::<Self>(info)
                    #description
                    .into_meta()
            }

            fn resolve(
                &self,
                info: &(),
                selection: Option<&[#crate_name::Selection<S>]>,
                executor: &#crate_name::Executor<Self::Context, S>,
            ) -> #crate_name::ExecutionResult<S> {
                #crate_name::GraphQLType::resolve(&self.0, info, selection, executor)
            }
        }

        impl<S> #crate_name::ToInputValue<S> for #ident
        where
            S: #crate_name::ScalarValue,
        {
            fn to_input_value(&self) -> #crate_name::InputValue<S> {
                let v = #resolve_body;
                #crate_name::ToInputValue::to_input_value(&v)
            }
        }

        impl<S> #crate_name::FromInputValue<S> for #ident
        where
            S: #crate_name::ScalarValue,
        {
            fn from_input_value(#from_input_value_arg: &#crate_name::InputValue<S>) -> #from_input_value_result {
                #from_input_value_body
            }
        }

        impl<S> #crate_name::ParseScalarValue<S> for #ident
        where
            S: #crate_name::ScalarValue,
        {
            fn from_str<'a>(
                #from_str_arg: #crate_name::parser::ScalarToken<'a>,
            ) -> #from_str_result {
                #from_str_body
            }
        }
    ).into()
}
