//! Parse impl blocks.

use proc_macro::TokenStream;
use quote::quote;

use crate::util;

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

impl ImplBlock {
    /// Parse a 'fn resolve()' metho declaration found in union or interface
    /// impl blocks.
    /// Returns the variable definitions needed for the resolve body.
    pub fn parse_resolve_method(
        &self,
        method: &syn::ImplItemMethod,
    ) -> Vec<proc_macro2::TokenStream> {
        if method.sig.ident != "resolve" {
            panic!("Expect a method named 'fn resolve(...)");
        }

        let _type = match &method.sig.output {
            syn::ReturnType::Type(_, _) => {
                panic!("resolve() method must not have a declared return type");
            }
            syn::ReturnType::Default => {}
        };

        let mut arguments = method.sig.inputs.iter();

        // Verify '&self' argument.
        match arguments.next() {
            Some(syn::FnArg::Receiver(rec)) => {
                if rec.reference.is_none() || rec.mutability.is_some() {
                    panic!(
                        "Invalid method receiver {}(self, ...): did you mean '&self'?",
                        method.sig.ident
                    );
                }
            }
            _ => {
                panic!("Expected a '&self' argument");
            }
        }

        let mut resolve_parts = Vec::new();

        for arg in arguments {
            match arg {
                syn::FnArg::Receiver(_) => {
                    panic!(
                        "Malformed method signature {}: self receiver must be the first argument",
                        method.sig.ident
                    );
                }
                syn::FnArg::Typed(captured) => {
                    let (arg_ident, _is_mut) = match &*captured.pat {
                        syn::Pat::Ident(ref pat_ident) => {
                            (&pat_ident.ident, pat_ident.mutability.is_some())
                        }
                        _ => {
                            panic!("Invalid token for function argument");
                        }
                    };
                    let context_type = self.attrs.context.as_ref();

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
                        panic!(
                            "Invalid context argument: to access the context, you need to specify the type as a reference.\nDid you mean &{}?",
                            quote!(captured.ty),
                        );
                    } else {
                        panic!("Invalid argument for 'resolve' method: only executor or context are allowed");
                    }
                }
            }
        }

        resolve_parts
    }

    pub fn parse(attr_tokens: TokenStream, body: TokenStream) -> ImplBlock {
        let attrs = match syn::parse::<util::ObjectAttributes>(attr_tokens) {
            Ok(attrs) => attrs,
            Err(e) => {
                panic!("Invalid attributes:\n{}", e);
            }
        };

        let mut _impl = match syn::parse::<syn::ItemImpl>(body) {
            Ok(item) => item,
            Err(err) => {
                panic!("Parsing error:\n{}", err);
            }
        };

        let target_trait = match _impl.trait_ {
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
            panic!("Could not determine a name for the impl type");
        };

        let target_type = _impl.self_ty;

        let description = attrs
            .description
            .clone()
            .or(util::get_doc_comment(&_impl.attrs));

        let mut methods = Vec::new();

        for item in _impl.items {
            match item {
                syn::ImplItem::Method(method) => {
                    methods.push(method);
                }
                _ => {
                    panic!("Invalid item for GraphQL Object: only type declarations and methods are allowed");
                }
            }
        }

        Self {
            attrs,
            type_ident,
            target_trait,
            target_type,
            generics: _impl.generics,
            description,
            methods,
        }
    }
}
