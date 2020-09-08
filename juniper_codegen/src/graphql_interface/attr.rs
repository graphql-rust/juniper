//! Code generation for `#[graphql_interface]` macro.

use std::mem;

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens as _};
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned};

use crate::{
    common::{
        anonymize_lifetimes,
        parse::{self, TypeExt as _},
    },
    result::GraphQLScope,
    util::{
        path_eq_single, span_container::SpanContainer, strip_attrs, to_camel_case, unite_attrs,
    },
};

use super::{
    ArgumentMeta, ImplementerDefinition, ImplementerDowncastDefinition, ImplementerMeta,
    InterfaceDefinition, InterfaceFieldArgumentDefinition, InterfaceFieldDefinition, InterfaceMeta,
    MethodArgument, TraitMethodMeta,
};

/// [`GraphQLScope`] of errors for `#[graphql_interface]` macro.
const ERR: GraphQLScope = GraphQLScope::InterfaceAttr;

/// Expands `#[graphql_interface]` macro into generated code.
pub fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemTrait>(body.clone()) {
        let trait_attrs = unite_attrs(("graphql_interface", &attr_args), &ast.attrs);
        ast.attrs = strip_attrs("graphql_interface", ast.attrs);
        return expand_on_trait(trait_attrs, ast);
    } else if let Ok(mut ast) = syn::parse2::<syn::ItemImpl>(body) {
        if ast.trait_.is_some() {
            let impl_attrs = unite_attrs(("graphql_interface", &attr_args), &ast.attrs);
            ast.attrs = strip_attrs("graphql_interface", ast.attrs);
            return expand_on_impl(impl_attrs, ast);
        }
    }

    Err(syn::Error::new(
        Span::call_site(),
        "#[graphql_interface] attribute is applicable to trait definitions and trait \
         implementations only",
    ))
}

/// Expands `#[graphql_interface]` macro placed on trait definition.
pub fn expand_on_trait(
    attrs: Vec<syn::Attribute>,
    mut ast: syn::ItemTrait,
) -> syn::Result<TokenStream> {
    let meta = InterfaceMeta::from_attrs("graphql_interface", &attrs)?;

    let trait_ident = &ast.ident;

    let name = meta
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| trait_ident.unraw().to_string());
    if !meta.is_internal && name.starts_with("__") {
        ERR.no_double_underscore(
            meta.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| trait_ident.span()),
        );
    }

    let mut implementers: Vec<_> = meta
        .implementers
        .iter()
        .map(|ty| {
            let span = ty.span_ident();
            ImplementerDefinition {
                ty: ty.as_ref().clone(),
                downcast: None,
                context_ty: None,
                span,
            }
        })
        .collect();
    for (ty, downcast) in &meta.external_downcasts {
        match implementers.iter_mut().find(|i| &i.ty == ty) {
            Some(impler) => {
                impler.downcast = Some(ImplementerDowncastDefinition::External {
                    path: downcast.inner().clone(),
                });
            }
            None => err_only_implementer_downcast(&downcast.span_joined()),
        }
    }

    proc_macro_error::abort_if_dirty();

    let mut fields = vec![];
    for item in &mut ast.items {
        if let syn::TraitItem::Method(m) = item {
            match TraitMethod::parse(m) {
                Some(TraitMethod::Field(f)) => fields.push(f),
                Some(TraitMethod::Downcast(d)) => {
                    match implementers.iter_mut().find(|i| i.ty == d.ty) {
                        Some(impler) => {
                            if let Some(external) = &impler.downcast {
                                err_duplicate_downcast(m, external, &impler.ty);
                            } else {
                                impler.downcast = d.downcast;
                                impler.context_ty = d.context_ty;
                            }
                        }
                        None => err_only_implementer_downcast(&d.span),
                    }
                }
                _ => {}
            }
        }
    }

    proc_macro_error::abort_if_dirty();

    let context = meta
        .context
        .map(SpanContainer::into_inner)
        .or_else(|| {
            fields.iter().find_map(|f| {
                f.arguments
                    .iter()
                    .find_map(MethodArgument::context_ty)
                    .cloned()
            })
        })
        .or_else(|| {
            implementers
                .iter()
                .find_map(|impler| impler.context_ty.as_ref())
                .cloned()
        });

    let is_async_trait = meta.asyncness.is_some()
        || ast
            .items
            .iter()
            .find_map(|item| match item {
                syn::TraitItem::Method(m) => m.sig.asyncness,
                _ => None,
            })
            .is_some();
    let has_default_async_methods = ast
        .items
        .iter()
        .find(|item| match item {
            syn::TraitItem::Method(m) => m.sig.asyncness.and(m.default.as_ref()).is_some(),
            _ => false,
        })
        .is_some();

    let generated_code = InterfaceDefinition {
        name,
        ty: parse_quote! { #trait_ident },
        trait_object: Some(meta.alias.map(|a| a.as_ref().clone())),
        visibility: ast.vis.clone(),
        description: meta.description.map(SpanContainer::into_inner),
        context,
        scalar: meta.scalar.map(SpanContainer::into_inner),
        generics: ast.generics.clone(),
        fields,
        implementers,
    };

    ast.generics.params.push(parse_quote! {
        GraphQLScalarValue: ::juniper::ScalarValue = ::juniper::DefaultScalarValue
    });
    ast.supertraits.push(parse_quote! {
        ::juniper::AsDynGraphQLValue<GraphQLScalarValue>
    });
    if is_async_trait && has_default_async_methods {
        // Hack for object safety. See details: https://docs.rs/async-trait/#dyn-traits
        ast.supertraits.push(parse_quote! { Sync });
    }
    ast.attrs
        .push(parse_quote! { #[allow(unused_qualifications, clippy::type_repetition_in_bounds)] });
    if is_async_trait {
        inject_async_trait(
            &mut ast.attrs,
            ast.items.iter_mut().filter_map(|i| {
                if let syn::TraitItem::Method(m) = i {
                    Some(&mut m.sig)
                } else {
                    None
                }
            }),
            &ast.generics,
        );
    }

    Ok(quote! {
        #ast

        #generated_code
    })
}

/// Expands `#[graphql_interface]` macro placed on trait implementation block.
pub fn expand_on_impl(
    attrs: Vec<syn::Attribute>,
    mut ast: syn::ItemImpl,
) -> syn::Result<TokenStream> {
    let meta = ImplementerMeta::from_attrs("graphql_interface", &attrs)?;

    let is_async_trait = meta.asyncness.is_some()
        || ast
            .items
            .iter()
            .find_map(|item| match item {
                syn::ImplItem::Method(m) => m.sig.asyncness,
                _ => None,
            })
            .is_some();

    let is_generic_scalar = meta.scalar.is_none();

    ast.attrs
        .push(parse_quote! { #[allow(unused_qualifications, clippy::type_repetition_in_bounds)] });

    if is_generic_scalar {
        ast.generics.params.push(parse_quote! {
            GraphQLScalarValue: ::juniper::ScalarValue + Send + Sync
        });
    }

    let (_, trait_path, _) = ast.trait_.as_mut().unwrap();
    let trait_params = &mut trait_path.segments.last_mut().unwrap().arguments;
    if let syn::PathArguments::None = trait_params {
        *trait_params = syn::PathArguments::AngleBracketed(parse_quote! { <> });
    }
    if let syn::PathArguments::AngleBracketed(a) = trait_params {
        a.args.push(if is_generic_scalar {
            parse_quote! { GraphQLScalarValue }
        } else {
            syn::GenericArgument::Type(meta.scalar.clone().unwrap().into_inner())
        });
    }

    if is_async_trait {
        inject_async_trait(
            &mut ast.attrs,
            ast.items.iter_mut().filter_map(|i| {
                if let syn::ImplItem::Method(m) = i {
                    Some(&mut m.sig)
                } else {
                    None
                }
            }),
            &ast.generics,
        );
    }

    Ok(quote! { #ast })
}

fn inject_async_trait<'m, M>(attrs: &mut Vec<syn::Attribute>, methods: M, generics: &syn::Generics)
where
    M: IntoIterator<Item = &'m mut syn::Signature>,
{
    attrs.push(parse_quote! { #[allow(clippy::type_repetition_in_bounds)] });
    attrs.push(parse_quote! { #[::juniper::async_trait] });

    for method in methods.into_iter() {
        if method.asyncness.is_some() {
            let where_clause = &mut method.generics.make_where_clause().predicates;
            for p in &generics.params {
                let ty_param = match p {
                    syn::GenericParam::Type(t) => {
                        let ty_param = &t.ident;
                        quote! { #ty_param }
                    }
                    syn::GenericParam::Lifetime(l) => {
                        let ty_param = &l.lifetime;
                        quote! { #ty_param }
                    }
                    syn::GenericParam::Const(_) => continue,
                };
                where_clause.push(parse_quote! { #ty_param: 'async_trait });
            }
        }
    }
}

enum TraitMethod {
    Field(InterfaceFieldDefinition),
    Downcast(ImplementerDefinition),
}

impl TraitMethod {
    fn parse(method: &mut syn::TraitItemMethod) -> Option<Self> {
        let method_attrs = method.attrs.clone();

        // Remove repeated attributes from the method, to omit incorrect expansion.
        method.attrs = mem::take(&mut method.attrs)
            .into_iter()
            .filter(|attr| !path_eq_single(&attr.path, "graphql_interface"))
            .collect();

        let meta = TraitMethodMeta::from_attrs("graphql_interface", &method_attrs)
            .map_err(|e| proc_macro_error::emit_error!(e))
            .ok()?;

        if meta.ignore.is_some() {
            return None;
        }

        if meta.downcast.is_some() {
            return Some(Self::Downcast(Self::parse_downcast(method)?));
        }

        Some(Self::Field(Self::parse_field(method, meta)?))
    }

    fn parse_downcast(method: &mut syn::TraitItemMethod) -> Option<ImplementerDefinition> {
        let method_span = method.sig.span();
        let method_ident = &method.sig.ident;

        let ty = parse::downcaster::output_type(&method.sig.output)
            .map_err(|span| {
                ERR.emit_custom(
                    span,
                    "expects trait method return type to be `Option<&ImplementerType>` only",
                )
            })
            .ok()?;
        let context_ty = parse::downcaster::context_ty(&method.sig)
            .map_err(|span| {
                ERR.emit_custom(
                    span,
                    "expects trait method to accept `&self` only and, optionally, `&Context`",
                )
            })
            .ok()?;
        if let Some(is_async) = &method.sig.asyncness {
            ERR.emit_custom(
                is_async.span(),
                "async downcast to interface implementer is not supported",
            );
            return None;
        }

        let downcast = ImplementerDowncastDefinition::Method {
            name: method_ident.clone(),
            with_context: context_ty.is_some(),
        };

        Some(ImplementerDefinition {
            ty,
            downcast: Some(downcast),
            context_ty,
            span: method_span,
        })
    }

    fn parse_field(
        method: &mut syn::TraitItemMethod,
        meta: TraitMethodMeta,
    ) -> Option<InterfaceFieldDefinition> {
        let method_ident = &method.sig.ident;

        let name = meta
            .name
            .as_ref()
            .map(|m| m.as_ref().value())
            .unwrap_or_else(|| to_camel_case(&method_ident.unraw().to_string()));
        if name.starts_with("__") {
            ERR.no_double_underscore(
                meta.name
                    .as_ref()
                    .map(SpanContainer::span_ident)
                    .unwrap_or_else(|| method_ident.span()),
            );
            return None;
        }

        let arguments = {
            if method.sig.inputs.is_empty() {
                return err_no_method_receiver(&method.sig.inputs);
            }
            let mut args_iter = method.sig.inputs.iter_mut();
            match args_iter.next().unwrap() {
                syn::FnArg::Receiver(rcv) => {
                    if !rcv.reference.is_some() || rcv.mutability.is_some() {
                        return err_invalid_method_receiver(rcv);
                    }
                }
                syn::FnArg::Typed(arg) => {
                    if let syn::Pat::Ident(a) = &*arg.pat {
                        if a.ident.to_string().as_str() != "self" {
                            return err_invalid_method_receiver(arg);
                        }
                    }
                    return err_no_method_receiver(arg);
                }
            };
            args_iter
                .filter_map(|arg| match arg {
                    syn::FnArg::Receiver(_) => None,
                    syn::FnArg::Typed(arg) => Self::parse_field_argument(arg),
                })
                .collect()
        };

        let mut ty = match &method.sig.output {
            syn::ReturnType::Default => parse_quote! { () },
            syn::ReturnType::Type(_, ty) => ty.unparenthesized().clone(),
        };
        anonymize_lifetimes(&mut ty);

        let description = meta.description.as_ref().map(|d| d.as_ref().value());
        let deprecated = meta
            .deprecated
            .as_ref()
            .map(|d| d.as_ref().as_ref().map(syn::LitStr::value));

        Some(InterfaceFieldDefinition {
            name,
            ty,
            description,
            deprecated,
            method: method_ident.clone(),
            arguments,
            is_async: method.sig.asyncness.is_some(),
        })
    }

    fn parse_field_argument(argument: &mut syn::PatType) -> Option<MethodArgument> {
        let argument_attrs = argument.attrs.clone();

        // Remove repeated attributes from the method, to omit incorrect expansion.
        argument.attrs = mem::take(&mut argument.attrs)
            .into_iter()
            .filter(|attr| !path_eq_single(&attr.path, "graphql_interface"))
            .collect();

        let meta = ArgumentMeta::from_attrs("graphql_interface", &argument_attrs)
            .map_err(|e| proc_macro_error::emit_error!(e))
            .ok()?;

        if meta.context.is_some() {
            return Some(MethodArgument::Context(argument.ty.unreferenced().clone()));
        }
        if meta.executor.is_some() {
            return Some(MethodArgument::Executor);
        }
        if let syn::Pat::Ident(name) = &*argument.pat {
            let arg = match name.ident.unraw().to_string().as_str() {
                "context" | "ctx" => {
                    Some(MethodArgument::Context(argument.ty.unreferenced().clone()))
                }
                "executor" => Some(MethodArgument::Executor),
                _ => None,
            };
            if arg.is_some() {
                ensure_no_regular_field_argument_meta(&meta)?;
                return arg;
            }
        }

        let name = if let Some(name) = meta.name.as_ref() {
            name.as_ref().value()
        } else if let syn::Pat::Ident(name) = &*argument.pat {
            to_camel_case(&name.ident.unraw().to_string())
        } else {
            ERR.custom(
                argument.pat.span(),
                "trait method argument should be declared as a single identifier",
            )
            .note(String::from(
                "use `#[graphql_interface(name = ...)]` attribute to specify custom argument's \
                 name without requiring it being a single identifier",
            ))
            .emit();
            return None;
        };
        if name.starts_with("__") {
            ERR.no_double_underscore(
                meta.name
                    .as_ref()
                    .map(SpanContainer::span_ident)
                    .unwrap_or_else(|| argument.pat.span()),
            );
            return None;
        }

        Some(MethodArgument::Regular(InterfaceFieldArgumentDefinition {
            name,
            ty: argument.ty.as_ref().clone(),
            description: meta.description.as_ref().map(|d| d.as_ref().value()),
            default: meta.default.as_ref().map(|v| v.as_ref().clone()),
        }))
    }
}

fn ensure_no_regular_field_argument_meta(meta: &ArgumentMeta) -> Option<()> {
    if let Some(span) = &meta.name {
        return err_disallowed_attr(&span, "name");
    }
    if let Some(span) = &meta.description {
        return err_disallowed_attr(&span, "description");
    }
    if let Some(span) = &meta.default {
        return err_disallowed_attr(&span, "default");
    }
    Some(())
}

fn err_disallowed_attr<T, S: Spanned>(span: &S, attr: &str) -> Option<T> {
    ERR.custom(
        span.span(),
        format!(
            "attribute `#[graphql_interface({} = ...)]` is not allowed here",
            attr,
        ),
    )
    .emit();
    return None;
}

fn err_invalid_method_receiver<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.custom(
        span.span(),
        "trait method receiver can only be a shared reference `&self`",
    )
    .emit();
    return None;
}

fn err_no_method_receiver<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.custom(
        span.span(),
        "trait method should have a shared reference receiver `&self`",
    )
    .emit();
    return None;
}

fn err_only_implementer_downcast<S: Spanned>(span: &S) {
    ERR.custom(
        span.span(),
        "downcasting is possible only to interface implementers",
    )
    .emit();
}

fn err_duplicate_downcast(
    method: &syn::TraitItemMethod,
    external: &ImplementerDowncastDefinition,
    impler_ty: &syn::Type,
) {
    let external = match external {
        ImplementerDowncastDefinition::External { path } => path,
        _ => unreachable!(),
    };

    ERR.custom(
        method.span(),
        format!(
            "trait method `{}` conflicts with the external downcast function `{}` declared \
             on the trait to downcast into the implementer type `{}`",
            method.sig.ident,
            external.to_token_stream(),
            impler_ty.to_token_stream(),
        ),
    )
    .note(String::from(
        "use `#[graphql_interface(ignore)]` attribute to ignore this trait method for interface \
         implementers downcasting",
    ))
    .emit()
}
