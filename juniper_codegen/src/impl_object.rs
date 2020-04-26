#![allow(clippy::collapsible_if)]

use crate::{
    result::GraphQLScope,
    util::{self, span_container::SpanContainer},
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

/// Generate code for the juniper::graphql_object macro.
pub fn build_object(
    args: TokenStream,
    body: TokenStream,
    is_internal: bool,
    error: GraphQLScope,
) -> TokenStream {
    let definition = match create(args, body, error) {
        Ok(definition) => definition,
        Err(err) => return err.to_compile_error(),
    };
    let juniper_crate_name = if is_internal { "crate" } else { "juniper" };

    definition.into_tokens(juniper_crate_name).into()
}

/// Generate code for the juniper::graphql_subscription macro.
pub fn build_subscription(
    args: TokenStream,
    body: TokenStream,
    is_internal: bool,
    error: GraphQLScope,
) -> TokenStream {
    let definition = match create(args, body, error) {
        Ok(definition) => definition,
        Err(err) => return err.to_compile_error(),
    };

    let juniper_crate_name = if is_internal { "crate" } else { "juniper" };
    definition
        .into_subscription_tokens(juniper_crate_name)
        .into()
}

fn create(
    args: TokenStream,
    body: TokenStream,
    error: GraphQLScope,
) -> syn::Result<util::GraphQLTypeDefiniton> {
    let body_span = body.span();
    let _impl = util::parse_impl::ImplBlock::parse(args, body)?;
    let name = _impl
        .attrs
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| _impl.type_ident.to_string());

    let context = _impl.attrs.context.map(SpanContainer::into_inner);

    let fields = _impl.methods
        .into_iter()
        .filter_map(|method| {
            let span = method.span();
            let _type = match &method.sig.output {
                syn::ReturnType::Type(_, ref t) => (**t).clone(),
                syn::ReturnType::Default => {
                    error.custom(method.sig.span(), "return value required");
                    return None
                }
            };

            let is_async = method.sig.asyncness.is_some();

            let attrs = match util::FieldAttributes::from_attrs(
                &method.attrs,
                util::FieldAttributeParseMode::Impl,
            ) {
                Ok(attrs) => attrs,
                Err(err) => {
                    proc_macro_error::emit_error!(err);
                    return None
                }
            };

            let mut args = Vec::new();
            let mut resolve_parts = Vec::new();

            for arg in method.sig.inputs {
                match arg {
                    syn::FnArg::Receiver(rec) => {
                        if rec.reference.is_none() || rec.mutability.is_some() {
                            error.custom(rec.span(), "expect &self");
                            // TODO: insert help: replace with &self?
                            return None
                        }
                    }
                    syn::FnArg::Typed(ref captured) => {
                        let (arg_ident, is_mut) = match &*captured.pat {
                            syn::Pat::Ident(ref pat_ident) => {
                                (&pat_ident.ident, pat_ident.mutability.is_some())
                            }
                            _ => {
                                error.custom(captured.pat.span(), "only single arguments are allowed (e.g., `test: String`)");
                                return None
                            }
                        };
                        let arg_name = arg_ident.to_string();

                        let context_type = context.as_ref();

                        // Check for executor arguments.
                        if util::type_is_identifier_ref(&captured.ty, "Executor") {
                            resolve_parts.push(quote!(let #arg_ident = executor;));
                        }
                        // Make sure executor is specified as a reference.
                        else if util::type_is_identifier(&captured.ty, "Executor") {
                            panic!("Invalid executor argument: to access the Executor, you need to specify the type as a reference.\nDid you mean &Executor?");
                        }
                        // Check for context arg.
                        else if context_type
                            .clone()
                            .map(|ctx| util::type_is_ref_of(&captured.ty, ctx))
                            .unwrap_or(false)
                        {

                            resolve_parts.push(quote!( let #arg_ident = executor.context(); ));
                        }
                        // Make sure the user does not specify the Context
                        //  without a reference. (&Context)
                        else if context_type
                            .clone()
                            .map(|ctx| ctx == &*captured.ty)
                            .unwrap_or(false)
                        {
                            error.custom(captured.ty.span(), format!("expected reference to context, but got `{:?}`", captured.ty));
                            // TODO: insert help: replace with &{}?
                            return None
                        } else {
                            // Regular argument.

                            let ty = &captured.ty;
                            // TODO: respect graphql attribute overwrite.
                            let final_name = util::to_camel_case(&arg_name);
                            let expect_text = format!(
                                "Internal error: missing argument {} - validation must have failed",
                                &final_name
                            );
                            let mut_modifier = if is_mut { quote!(mut) } else { quote!() };
                            resolve_parts.push(quote!(
                                let #mut_modifier #arg_ident = args
                                    .get::<#ty>(#final_name)
                                    .expect(#expect_text);
                            ));
                            args.push(util::GraphQLTypeDefinitionFieldArg {
                                description: attrs
                                    .argument(&arg_name)
                                    .and_then(|arg| arg.description.as_ref().map(|d| d.value())),
                                default: attrs
                                    .argument(&arg_name)
                                    .and_then(|arg| arg.default.clone()),
                                _type: ty.clone(),
                                name: final_name,
                            })
                        }
                    }
                }
            }

            let body = &method.block;
            let resolver_code = quote!(
                #( #resolve_parts )*
                #body
            );

            let ident = &method.sig.ident;
            let name = attrs
                .name
                .map(SpanContainer::into_inner)
                .unwrap_or_else(|| util::to_camel_case(&ident.to_string()));

            Some(util::GraphQLTypeDefinitionField {
                name,
                _type,
                args,
                description: attrs.description.map(SpanContainer::into_inner),
                deprecation: attrs.deprecation.map(SpanContainer::into_inner),
                resolver_code,
                is_type_inferred: false,
                is_async,
                span,
            })
        }).collect::<Vec<_>>();

    // Early abort after checking all fields
    proc_macro_error::abort_if_dirty();

    if fields.is_empty() {
        error.not_empty(body_span);
    }

    match crate::util::duplicate::Duplicate::find_by_key(
        &fields,
        |field| &field.name,
    ) {
        Some(duplicates) => error.duplicate(duplicates.iter()),
        None => {}
    }

    // Early abort after GraphQL properties
    proc_macro_error::abort_if_dirty();

    let definition = util::GraphQLTypeDefiniton {
        name,
        _type: *_impl.target_type.clone(),
        scalar: _impl.attrs.scalar.map(SpanContainer::into_inner),
        context,
        description: _impl.description,
        fields,
        generics: _impl.generics.clone(),
        interfaces: if !_impl.attrs.interfaces.is_empty() {
            Some(
                _impl
                    .attrs
                    .interfaces
                    .into_iter()
                    .map(SpanContainer::into_inner)
                    .collect(),
            )
        } else {
            None
        },
        include_type_generics: false,
        generic_scalar: false,
        no_async: _impl.attrs.no_async.is_some(),
    };

    Ok(definition)
}
