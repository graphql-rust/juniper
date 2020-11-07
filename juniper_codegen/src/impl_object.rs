#![allow(clippy::collapsible_if)]

use crate::{
    result::{GraphQLScope, UnsupportedAttribute},
    util::{self, span_container::SpanContainer, RenameRule},
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ext::IdentExt, spanned::Spanned};

/// Generate code for the juniper::graphql_object macro.
pub fn build_object(args: TokenStream, body: TokenStream, error: GraphQLScope) -> TokenStream {
    let definition = match create(args, body, error) {
        Ok(definition) => definition,
        Err(err) => return err.to_compile_error(),
    };
    definition.into_tokens()
}

/// Generate code for the juniper::graphql_subscription macro.
pub fn build_subscription(
    args: TokenStream,
    body: TokenStream,
    error: GraphQLScope,
) -> TokenStream {
    let definition = match create(args, body, error) {
        Ok(definition) => definition,
        Err(err) => return err.to_compile_error(),
    };
    definition.into_subscription_tokens()
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
        .unwrap_or_else(|| _impl.type_ident.unraw().to_string());

    let top_attrs = &_impl.attrs;

    let scalar = _impl
        .attrs
        .scalar
        .as_ref()
        .map(|s| quote!( #s ))
        .unwrap_or_else(|| quote!(::juniper::DefaultScalarValue));

    let fields = _impl
        .methods
        .iter()
        .filter_map(|method| {
            let span = method.span();
            let _type = match method.sig.output {
                syn::ReturnType::Type(_, ref t) => *t.clone(),
                syn::ReturnType::Default => {
                    error.emit_custom(method.sig.span(), "return value required");
                    return None;
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
                    return None;
                }
            };

            let parse_method =
                _impl.parse_method(&method, true, |captured, arg_ident, is_mut: bool| {
                    let arg_name = arg_ident.unraw().to_string();
                    let ty = &captured.ty;

                    let final_name = attrs
                        .argument(&arg_name)
                        .and_then(|attrs| attrs.rename.clone().map(|ident| ident.value()))
                        .unwrap_or_else(|| {
                            top_attrs
                                .rename
                                .unwrap_or(RenameRule::CamelCase)
                                .apply(&arg_name)
                        });

                    let mut_modifier = if is_mut { quote!(mut) } else { quote!() };

                    if final_name.starts_with("__") {
                        error.no_double_underscore(
                            if let Some(name) = attrs
                                .argument(&arg_name)
                                .and_then(|attrs| attrs.rename.as_ref())
                            {
                                name.span_ident()
                            } else {
                                arg_ident.span()
                            },
                        );
                    }

                    let resolver = quote!(
                        let #mut_modifier #arg_ident = args
                            .get::<#ty>(#final_name)
                            .unwrap_or_else(::juniper::FromInputValue::<#scalar>::from_implicit_null);
                    );

                    let field_type = util::GraphQLTypeDefinitionFieldArg {
                        description: attrs
                            .argument(&arg_name)
                            .and_then(|arg| arg.description.as_ref().map(|d| d.value())),
                        default: attrs
                            .argument(&arg_name)
                            .and_then(|arg| arg.default.clone()),
                        _type: ty.clone(),
                        name: final_name,
                    };
                    Ok((resolver, field_type))
                });

            let (resolve_parts, args) = match parse_method {
                Ok((resolve_parts, args)) => (resolve_parts, args),
                Err(err) => {
                    proc_macro_error::emit_error!(err);
                    return None;
                }
            };

            let body = &method.block;
            let resolver_code = quote!(
                #( #resolve_parts )*
                #body
            );

            let ident = &method.sig.ident;
            let name = attrs
                .name
                .clone()
                .map(SpanContainer::into_inner)
                .unwrap_or_else(|| {
                    top_attrs
                        .rename
                        .unwrap_or(RenameRule::CamelCase)
                        .apply(&ident.unraw().to_string())
                });

            if name.starts_with("__") {
                error.no_double_underscore(if let Some(name) = attrs.name {
                    name.span_ident()
                } else {
                    ident.span()
                });
            }

            if let Some(default) = attrs.default {
                error.unsupported_attribute_within(
                    default.span_ident(),
                    UnsupportedAttribute::Default,
                );
            }

            Some(util::GraphQLTypeDefinitionField {
                name,
                _type,
                args,
                description: attrs.description.map(SpanContainer::into_inner),
                deprecation: attrs.deprecation.map(SpanContainer::into_inner),
                resolver_code,
                is_type_inferred: false,
                is_async,
                default: None,
                span,
            })
        })
        .collect::<Vec<_>>();

    // Early abort after checking all fields
    proc_macro_error::abort_if_dirty();

    if let Some(duplicates) =
        crate::util::duplicate::Duplicate::find_by_key(&fields, |field| &field.name)
    {
        error.duplicate(duplicates.iter())
    }

    if !_impl.attrs.is_internal && name.starts_with("__") {
        error.no_double_underscore(if let Some(name) = _impl.attrs.name {
            name.span_ident()
        } else {
            _impl.type_ident.span()
        });
    }

    if fields.is_empty() {
        error.not_empty(body_span);
    }

    // Early abort after GraphQL properties
    proc_macro_error::abort_if_dirty();

    let definition = util::GraphQLTypeDefiniton {
        name,
        _type: *_impl.target_type.clone(),
        scalar: _impl.attrs.scalar.map(SpanContainer::into_inner),
        context: _impl.attrs.context.map(SpanContainer::into_inner),
        description: _impl.description,
        fields,
        generics: _impl.generics.clone(),
        interfaces: _impl
            .attrs
            .interfaces
            .into_iter()
            .map(SpanContainer::into_inner)
            .collect(),
        include_type_generics: false,
        generic_scalar: true,
        no_async: _impl.attrs.no_async.is_some(),
    };

    Ok(definition)
}
