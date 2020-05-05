//! Parse impl blocks.
#![allow(clippy::or_fun_call)]

use crate::util::{self, span_container::SpanContainer};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::convert::From;
use syn::{spanned::Spanned, PatType};

pub struct ImplBlock {
    pub attrs: util::ObjectAttributes,
    pub target_trait: Option<(String, syn::Path)>,
    pub target_type: Box<syn::Type>,
    pub type_ident: syn::Ident,
    pub generics: syn::Generics,
    // _impl: syn::ItemImpl,
    pub methods: Vec<syn::ImplItemMethod>,
    pub description: Option<String>,
}

struct ImplItemMethods(Vec<syn::ImplItemMethod>);

impl syn::parse::Parse for ImplItemMethods {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let mut vec= Vec::with_capacity(2);
        while let Ok(method) = input.parse::<syn::ItemFn>() {
            vec.push(syn::ImplItemMethod {
                attrs: method.attrs,
                vis: method.vis,
                defaultness: None,
                sig: method.sig,
                block: *method.block,
            });
        }
        while let Ok(method) = input.parse::<syn::ImplItemMethod>() {
            vec.push(method);
        }
        Ok(ImplItemMethods(vec))
    }
}

impl ImplBlock {
    /// Parse a `fn resolve()` method declaration found in most
    /// generators which rely on `impl` blocks.
    pub fn parse_resolve_method(
        &self,
        method: &syn::ImplItemMethod,
    ) -> syn::Result<Vec<TokenStream>> {
        if method.sig.ident != "resolve" {
            return Err(syn::Error::new(
                method.sig.ident.span(),
                "expect the method named `resolve`",
            ));
        }

        if let syn::ReturnType::Type(_, _) = &method.sig.output {
            return Err(syn::Error::new(
                method.sig.output.span(),
                "method must not have a declared return type",
            ));
        }

        //NOTICE: `fn resolve()` is a subset of `fn <NAME>() -> <TYPE>`
        self.parse_method(method, false, |captured, _, _| {
            Err(syn::Error::new(
                captured.span(),
                "only executor or context types are allowed",
            ))
        })
        .map(|(tokens, _empty)| tokens)
    }

    /// Parse a `fn <NAME>() -> <TYPE>` method declaration found in
    /// objects.
    pub fn parse_method<
        F: Fn(
            &PatType,
            &Ident,
            bool,
        ) -> syn::Result<(TokenStream, util::GraphQLTypeDefinitionFieldArg)>,
    >(
        &self,
        method: &syn::ImplItemMethod,
        is_self_optional: bool,
        f: F,
    ) -> syn::Result<(Vec<TokenStream>, Vec<util::GraphQLTypeDefinitionFieldArg>)> {
        let mut arguments = method.sig.inputs.iter().peekable();

        // Verify `&self` argument.
        match arguments.peek() {
            Some(syn::FnArg::Receiver(rec)) => {
                let _consume = arguments.next();
                if rec.reference.is_none() || rec.mutability.is_some() {
                    return Err(syn::Error::new(
                        rec.span(),
                        "invalid argument: did you mean `&self`?",
                    ));
                }
            }
            _ => {
                if !is_self_optional {
                    return Err(syn::Error::new(
                        method.sig.span(),
                        "expected a `&self` argument",
                    ));
                }
            }
        }

        let mut resolve_parts = Vec::new();
        let mut additional_arguments = Vec::new();

        for arg in arguments {
            match arg {
                syn::FnArg::Receiver(_) => {
                    if !is_self_optional {
                        return Err(syn::Error::new(
                            method.sig.ident.span(),
                            "self receiver must be the first argument",
                        ));
                    }
                }
                syn::FnArg::Typed(captured) => {
                    let (arg_ident, is_mut) = match &*captured.pat {
                        syn::Pat::Ident(ref pat_ident) => {
                            (&pat_ident.ident, pat_ident.mutability.is_some())
                        }
                        _ => {
                            return Err(syn::Error::new(
                                captured.pat.span(),
                                "expected identifier for function argument",
                            ));
                        }
                    };
                    let context_type = self.attrs.context.as_ref();

                    // Check for executor arguments.
                    if util::type_is_identifier_ref(&captured.ty, "Executor") {
                        resolve_parts.push(quote!(let #arg_ident = executor;));
                    }
                    // Make sure executor is specified as a reference.
                    else if util::type_is_identifier(&captured.ty, "Executor") {
                        return Err(syn::Error::new(
                            captured.ty.span(),
                            "to access the Executor, you need to specify the type as a reference.\nDid you mean &Executor?"
                        ));
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
                        .map(|ctx| ctx.inner() == &*captured.ty)
                        .unwrap_or(false)
                    {
                        return Err(syn::Error::new(
                            captured.ty.span(),
                            format!("to access the context, you need to specify the type as a reference.\nDid you mean &{}?", quote!(captured.ty)),
                        ));
                    } else {
                        let (tokens, ty) = f(captured, arg_ident, is_mut)?;
                        resolve_parts.push(tokens);
                        additional_arguments.push(ty);
                    }
                }
            }
        }

        Ok((resolve_parts, additional_arguments))
    }

    pub fn parse(attr_tokens: TokenStream, body: TokenStream) -> syn::Result<ImplBlock> {
        let attrs = syn::parse2::<util::ObjectAttributes>(attr_tokens)?;
        let mut _impl = syn::parse2::<syn::ItemImpl>(body)?;

        let target_trait = match _impl.clone().trait_ {
            Some((_, path, _)) => {
                let name = path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join(".");
                Some((name, path))
            }
            None => None,
        };

        let type_ident = if let Some(ident) = util::name_of_type(&*_impl.self_ty) {
            ident
        } else {
            return Err(syn::Error::new(
                _impl.self_ty.span(),
                "could not determine a name for the impl type",
            ));
        };

        let target_type = _impl.self_ty.clone();

        let description = attrs
            .description
            .clone()
            .or_else(|| util::get_doc_comment(&_impl.attrs.clone()));

        let mut methods = Vec::new();

        let error = |item: syn::ImplItem| {
            return Err(syn::Error::new(
                item.span(),
                "only type declarations and methods are allowed",
            ));
        };

        for item in _impl.items {
            match item {
                syn::ImplItem::Macro(mac) => {
                    for method in syn::parse2::<ImplItemMethods>(mac.mac.tokens)?.0 {
                        methods.push(method);
                    }
                }
                syn::ImplItem::Method(method) => methods.push(method),
                _ => error(item)?
            }
        }

        Ok(Self {
            attrs,
            type_ident,
            target_trait,
            target_type,
            generics: _impl.generics,
            description: description.map(SpanContainer::into_inner),
            methods,
        })
    }
}
