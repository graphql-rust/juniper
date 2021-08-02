//! Code generation for `#[graphql_interface]` macro.

use std::mem;

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens as _};
use syn::{ext::IdentExt as _, parse_quote, spanned::Spanned};

use crate::{
    common::{
        field,
        parse::{self, TypeExt as _},
        scalar,
    },
    result::GraphQLScope,
    util::{path_eq_single, span_container::SpanContainer, RenameRule},
};

use super::{
    inject_async_trait, Definition, EnumType, ImplAttr, Implementer, ImplementerDowncast,
    TraitAttr, TraitObjectType, Type,
};

/// [`GraphQLScope`] of errors for `#[graphql_interface]` macro.
const ERR: GraphQLScope = GraphQLScope::InterfaceAttr;

/// Expands `#[graphql_interface]` macro into generated code.
pub fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemTrait>(body.clone()) {
        let trait_attrs = parse::attr::unite(("graphql_interface", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip("graphql_interface", ast.attrs);
        return expand_on_trait(trait_attrs, ast);
    } else if let Ok(mut ast) = syn::parse2::<syn::ItemImpl>(body) {
        if ast.trait_.is_some() {
            let impl_attrs = parse::attr::unite(("graphql_interface", &attr_args), &ast.attrs);
            ast.attrs = parse::attr::strip("graphql_interface", ast.attrs);
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
fn expand_on_trait(
    attrs: Vec<syn::Attribute>,
    mut ast: syn::ItemTrait,
) -> syn::Result<TokenStream> {
    let attr = TraitAttr::from_attrs("graphql_interface", &attrs)?;

    let trait_ident = &ast.ident;
    let trait_span = ast.span();

    let name = attr
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| trait_ident.unraw().to_string());
    if !attr.is_internal && name.starts_with("__") {
        ERR.no_double_underscore(
            attr.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| trait_ident.span()),
        );
    }

    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    let mut implementers: Vec<_> = attr
        .implementers
        .iter()
        .map(|ty| Implementer {
            ty: ty.as_ref().clone(),
            downcast: None,
            context: None,
            scalar: scalar.clone(),
        })
        .collect();
    for (ty, downcast) in &attr.external_downcasts {
        match implementers.iter_mut().find(|i| &i.ty == ty) {
            Some(impler) => {
                impler.downcast = Some(ImplementerDowncast::External {
                    path: downcast.inner().clone(),
                });
            }
            None => err_only_implementer_downcast(&downcast.span_joined()),
        }
    }

    proc_macro_error::abort_if_dirty();

    let renaming = attr
        .rename_fields
        .as_deref()
        .copied()
        .unwrap_or(RenameRule::CamelCase);

    let mut fields = vec![];
    for item in &mut ast.items {
        if let syn::TraitItem::Method(m) = item {
            match TraitMethod::parse(m, &renaming) {
                Some(TraitMethod::Field(f)) => fields.push(f),
                Some(TraitMethod::Downcast(d)) => {
                    match implementers.iter_mut().find(|i| i.ty == d.ty) {
                        Some(impler) => {
                            if let Some(external) = &impler.downcast {
                                err_duplicate_downcast(m, external, &impler.ty);
                            } else {
                                impler.downcast = d.downcast;
                                impler.context = d.context;
                            }
                        }
                        None => err_only_implementer_downcast(&m.sig),
                    }
                }
                _ => {}
            }
        }
    }

    proc_macro_error::abort_if_dirty();

    if fields.is_empty() {
        ERR.emit_custom(trait_span, "must have at least one field");
    }
    if !field::all_different(&fields) {
        ERR.emit_custom(trait_span, "must have a different name for each field");
    }

    proc_macro_error::abort_if_dirty();

    let context = attr
        .context
        .as_deref()
        .cloned()
        .or_else(|| {
            fields.iter().find_map(|f| {
                f.arguments.as_ref().and_then(|f| {
                    f.iter()
                        .find_map(field::MethodArgument::context_ty)
                        .cloned()
                })
            })
        })
        .or_else(|| {
            implementers
                .iter()
                .find_map(|impler| impler.context.as_ref())
                .cloned()
        })
        .unwrap_or_else(|| parse_quote! { () });

    let is_trait_object = attr.r#dyn.is_some();

    let is_async_trait = attr.asyncness.is_some()
        || ast
            .items
            .iter()
            .find_map(|item| match item {
                syn::TraitItem::Method(m) => m.sig.asyncness,
                _ => None,
            })
            .is_some();
    let has_default_async_methods = ast.items.iter().any(|item| match item {
        syn::TraitItem::Method(m) => m.sig.asyncness.and(m.default.as_ref()).is_some(),
        _ => false,
    });

    let ty = if is_trait_object {
        Type::TraitObject(Box::new(TraitObjectType::new(
            &ast,
            &attr,
            scalar.clone(),
            context.clone(),
        )))
    } else {
        Type::Enum(Box::new(EnumType::new(
            &ast,
            &attr,
            &implementers,
            scalar.clone(),
        )))
    };

    let generated_code = Definition {
        ty,

        name,
        description: attr.description.map(SpanContainer::into_inner),

        context,
        scalar: scalar.clone(),

        fields,
        implementers,
    };

    // Attach the `juniper::AsDynGraphQLValue` on top of the trait if dynamic dispatch is used.
    if is_trait_object {
        ast.attrs.push(parse_quote! {
            #[allow(unused_qualifications, clippy::type_repetition_in_bounds)]
        });

        let scalar_ty = scalar.generic_ty();
        if !scalar.is_explicit_generic() {
            let default_ty = scalar.default_ty();
            ast.generics
                .params
                .push(parse_quote! { #scalar_ty = #default_ty });
        }
        ast.generics
            .make_where_clause()
            .predicates
            .push(parse_quote! { #scalar_ty: ::juniper::ScalarValue });
        ast.supertraits
            .push(parse_quote! { ::juniper::AsDynGraphQLValue<#scalar_ty> });
    }

    if is_async_trait {
        if has_default_async_methods {
            // Hack for object safety. See details: https://docs.rs/async-trait/#dyn-traits
            ast.supertraits.push(parse_quote! { Sync });
        }
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

/// Expands `#[graphql_interface]` macro placed on a trait implementation block.
fn expand_on_impl(attrs: Vec<syn::Attribute>, mut ast: syn::ItemImpl) -> syn::Result<TokenStream> {
    let attr = ImplAttr::from_attrs("graphql_interface", &attrs)?;

    let is_async_trait = attr.asyncness.is_some()
        || ast
            .items
            .iter()
            .find_map(|item| match item {
                syn::ImplItem::Method(m) => m.sig.asyncness,
                _ => None,
            })
            .is_some();

    let is_trait_object = attr.r#dyn.is_some();

    if is_trait_object {
        let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

        ast.attrs.push(parse_quote! {
            #[allow(unused_qualifications, clippy::type_repetition_in_bounds)]
        });

        if scalar.is_implicit_generic() {
            ast.generics.params.push(parse_quote! { #scalar });
        }
        if scalar.is_generic() {
            ast.generics
                .make_where_clause()
                .predicates
                .push(parse_quote! { #scalar: ::juniper::ScalarValue + Send + Sync });
        }

        if !scalar.is_explicit_generic() {
            let (_, trait_path, _) = ast.trait_.as_mut().unwrap();
            let trait_params = &mut trait_path.segments.last_mut().unwrap().arguments;
            if let syn::PathArguments::None = trait_params {
                *trait_params = syn::PathArguments::AngleBracketed(parse_quote! { <> });
            }
            if let syn::PathArguments::AngleBracketed(a) = trait_params {
                a.args.push(parse_quote! { #scalar });
            }
        }
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

/// Representation of parsed Rust trait method for `#[graphql_interface]` macro code generation.
enum TraitMethod {
    /// Method represents a [`Field`] of [GraphQL interface][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    Field(field::Definition),

    /// Method represents a custom downcasting function into the [`Implementer`] of
    /// [GraphQL interface][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Interfaces
    Downcast(Box<Implementer>),
}

impl TraitMethod {
    /// Parses this [`TraitMethod`] from the given trait method definition.
    ///
    /// Returns [`None`] if the trait method marked with `#[graphql(ignore)]` attribute,
    /// or parsing fails.
    #[must_use]
    fn parse(method: &mut syn::TraitItemMethod, renaming: &RenameRule) -> Option<Self> {
        let method_attrs = method.attrs.clone();

        // Remove repeated attributes from the method, to omit incorrect expansion.
        method.attrs = mem::take(&mut method.attrs)
            .into_iter()
            .filter(|attr| !path_eq_single(&attr.path, "graphql"))
            .collect();

        let attr = field::Attr::from_attrs("graphql", &method_attrs)
            .map_err(|e| proc_macro_error::emit_error!(e))
            .ok()?;

        if attr.ignore.is_some() {
            return None;
        }

        if attr.downcast.is_some() {
            return Some(Self::Downcast(Box::new(Self::parse_downcast(method)?)));
        }

        Some(Self::Field(Self::parse_field(method, attr, renaming)?))
    }

    /// Parses [`TraitMethod::Downcast`] from the given trait method definition.
    ///
    /// Returns [`None`] if parsing fails.
    #[must_use]
    fn parse_downcast(method: &mut syn::TraitItemMethod) -> Option<Implementer> {
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

        let downcast = ImplementerDowncast::Method {
            name: method_ident.clone(),
            with_context: context_ty.is_some(),
        };

        Some(Implementer {
            ty,
            downcast: Some(downcast),
            context: context_ty,
            scalar: scalar::Type::ImplicitGeneric(None),
        })
    }

    /// Parses [`TraitMethod::Field`] from the given trait method definition.
    ///
    /// Returns [`None`] if parsing fails.
    #[must_use]
    fn parse_field(
        method: &mut syn::TraitItemMethod,
        attr: field::Attr,
        renaming: &RenameRule,
    ) -> Option<field::Definition> {
        let method_ident = &method.sig.ident;

        let name = attr
            .name
            .as_ref()
            .map(|m| m.as_ref().value())
            .unwrap_or_else(|| renaming.apply(&method_ident.unraw().to_string()));
        if name.starts_with("__") {
            ERR.no_double_underscore(
                attr.name
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
                    if rcv.reference.is_none() || rcv.mutability.is_some() {
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
                    syn::FnArg::Typed(arg) => field::MethodArgument::parse(arg, renaming, &ERR),
                })
                .collect()
        };

        let mut ty = match &method.sig.output {
            syn::ReturnType::Default => parse_quote! { () },
            syn::ReturnType::Type(_, ty) => ty.unparenthesized().clone(),
        };
        ty.lifetimes_anonymized();

        let description = attr.description.as_ref().map(|d| d.as_ref().value());
        let deprecated = attr
            .deprecated
            .as_deref()
            .map(|d| d.as_ref().map(syn::LitStr::value));

        Some(field::Definition {
            name,
            ty,
            description,
            deprecated,
            ident: method_ident.clone(),
            arguments: Some(arguments),
            has_receiver: method.sig.receiver().is_some(),
            is_async: method.sig.asyncness.is_some(),
        })
    }
}

/// Emits "invalid trait method receiver" [`syn::Error`] pointing to the given
/// `span`.
#[must_use]
fn err_invalid_method_receiver<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.emit_custom(
        span.span(),
        "trait method receiver can only be a shared reference `&self`",
    );
    None
}

/// Emits "no trait method receiver" [`syn::Error`] pointing to the given
/// `span`.
#[must_use]
fn err_no_method_receiver<T, S: Spanned>(span: &S) -> Option<T> {
    ERR.emit_custom(
        span.span(),
        "trait method should have a shared reference receiver `&self`",
    );
    None
}

/// Emits "non-implementer downcast target" [`syn::Error`] pointing to the given
/// `span`.
fn err_only_implementer_downcast<S: Spanned>(span: &S) {
    ERR.emit_custom(
        span.span(),
        "downcasting is possible only to interface implementers",
    );
}

/// Emits "duplicate downcast" [`syn::Error`] for the given `method` and
/// `external` [`ImplementerDowncast`] function.
fn err_duplicate_downcast(
    method: &syn::TraitItemMethod,
    external: &ImplementerDowncast,
    impler_ty: &syn::Type,
) {
    let external = match external {
        ImplementerDowncast::External { path } => path,
        _ => unreachable!(),
    };

    ERR.custom(
        method.span(),
        format!(
            "trait method `{}` conflicts with the external downcast function \
             `{}` declared on the trait to downcast into the implementer type \
             `{}`",
            method.sig.ident,
            external.to_token_stream(),
            impler_ty.to_token_stream(),
        ),
    )
    .note(String::from(
        "use `#[graphql(ignore)]` attribute argument to ignore this trait \
         method for interface implementers downcasting",
    ))
    .emit()
}
