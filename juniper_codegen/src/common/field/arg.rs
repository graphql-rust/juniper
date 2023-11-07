//! Common functions, definitions and extensions for parsing and code generation
//! of [GraphQL arguments][1]
//!
//! [1]: https://spec.graphql.org/October2021#sec-Language.Arguments.

use std::mem;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    ext::IdentExt as _,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token,
};

use crate::common::{
    default, diagnostic, filter_attrs,
    parse::{
        attr::{err, OptionExt as _},
        ParseBufferExt as _, TypeExt as _,
    },
    path_eq_single, rename, scalar, Description, SpanContainer,
};

/// Available metadata (arguments) behind `#[graphql]` attribute placed on a
/// method argument, when generating code for [GraphQL argument][1].
///
/// [1]: https://spec.graphql.org/October2021#sec-Language.Arguments
#[derive(Debug, Default)]
pub(crate) struct Attr {
    /// Explicitly specified name of a [GraphQL argument][1] represented by this
    /// method argument.
    ///
    /// If [`None`], then `camelCased` Rust argument name is used by default.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Arguments
    pub(crate) name: Option<SpanContainer<syn::LitStr>>,

    /// Explicitly specified [description][2] of this [GraphQL argument][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Arguments
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    pub(crate) description: Option<SpanContainer<Description>>,

    /// Explicitly specified [default value][2] of this [GraphQL argument][1].
    ///
    /// If [`None`], then this [GraphQL argument][1] is considered as
    /// [required][2].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Arguments
    /// [2]: https://spec.graphql.org/October2021#sec-Required-Arguments
    pub(crate) default: Option<SpanContainer<default::Value>>,

    /// Explicitly specified marker indicating that this method argument doesn't
    /// represent a [GraphQL argument][1], but is a [`Context`] being injected
    /// into a [GraphQL field][2] resolving function.
    ///
    /// If absent, then the method argument still is considered as [`Context`]
    /// if it's named `context` or `ctx`.
    ///
    /// [`Context`]: juniper::Context
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Arguments
    /// [2]: https://spec.graphql.org/October2021#sec-Language.Fields
    pub(crate) context: Option<SpanContainer<syn::Ident>>,

    /// Explicitly specified marker indicating that this method argument doesn't
    /// represent a [GraphQL argument][1], but is an [`Executor`] being injected
    /// into a [GraphQL field][2] resolving function.
    ///
    /// If absent, then the method argument still is considered as [`Executor`]
    /// if it's named `executor`.
    ///
    /// [`Executor`]: juniper::Executor
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Arguments
    /// [2]: https://spec.graphql.org/October2021#sec-Language.Fields
    pub(crate) executor: Option<SpanContainer<syn::Ident>>,
}

impl Parse for Attr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut out = Self::default();
        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;
            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<token::Eq>()?;
                    let name = input.parse::<syn::LitStr>()?;
                    out.name
                        .replace(SpanContainer::new(ident.span(), Some(name.span()), name))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "desc" | "description" => {
                    input.parse::<token::Eq>()?;
                    let desc = input.parse::<Description>()?;
                    out.description
                        .replace(SpanContainer::new(ident.span(), Some(desc.span()), desc))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "default" => {
                    let val = input.parse::<default::Value>()?;
                    out.default
                        .replace(SpanContainer::new(ident.span(), Some(val.span()), val))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "ctx" | "context" | "Context" => {
                    let span = ident.span();
                    out.context
                        .replace(SpanContainer::new(span, Some(span), ident))
                        .none_or_else(|_| err::dup_arg(span))?
                }
                "exec" | "executor" => {
                    let span = ident.span();
                    out.executor
                        .replace(SpanContainer::new(span, Some(span), ident))
                        .none_or_else(|_| err::dup_arg(span))?
                }
                name => {
                    return Err(err::unknown_arg(&ident, name));
                }
            }
            input.try_parse::<token::Comma>()?;
        }
        Ok(out)
    }
}

impl Attr {
    /// Tries to merge two [`Attr`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            default: try_merge_opt!(default: self, another),
            context: try_merge_opt!(context: self, another),
            executor: try_merge_opt!(executor: self, another),
        })
    }

    /// Parses [`Attr`] from the given multiple `name`d [`syn::Attribute`]s
    /// placed on a function argument.
    pub(crate) fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let attr = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if let Some(context) = &attr.context {
            if attr.name.is_some()
                || attr.description.is_some()
                || attr.default.is_some()
                || attr.executor.is_some()
            {
                return Err(syn::Error::new(
                    context.span(),
                    "`context` attribute argument is not composable with any other arguments",
                ));
            }
        }

        if let Some(executor) = &attr.executor {
            if attr.name.is_some()
                || attr.description.is_some()
                || attr.default.is_some()
                || attr.context.is_some()
            {
                return Err(syn::Error::new(
                    executor.span(),
                    "`executor` attribute argument is not composable with any other arguments",
                ));
            }
        }

        Ok(attr)
    }

    /// Checks whether this [`Attr`] doesn't contain arguments related to an
    /// [`OnField`] argument.
    fn ensure_no_regular_arguments(&self) -> syn::Result<()> {
        if let Some(span) = &self.name {
            return Err(Self::err_disallowed(&span, "name"));
        }
        if let Some(span) = &self.description {
            return Err(Self::err_disallowed(&span, "description"));
        }
        if let Some(span) = &self.default {
            return Err(Self::err_disallowed(&span, "default"));
        }
        Ok(())
    }

    /// Emits "argument is not allowed" [`syn::Error`] for the given `arg`
    /// pointing to the given `span`.
    #[must_use]
    fn err_disallowed<S: Spanned>(span: &S, arg: &str) -> syn::Error {
        syn::Error::new(
            span.span(),
            format!("attribute argument `#[graphql({arg} = ...)]` is not allowed here",),
        )
    }
}

/// Representation of a [GraphQL field argument][1] for code generation.
///
/// [1]: https://spec.graphql.org/October2021#sec-Language.Arguments
#[derive(Debug)]
pub(crate) struct OnField {
    /// Rust type that this [GraphQL field argument][1] is represented by.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Arguments
    pub(crate) ty: syn::Type,

    /// Name of this [GraphQL field argument][2] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Arguments
    pub(crate) name: String,

    /// [Description][2] of this [GraphQL field argument][1] to put into GraphQL
    /// schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Arguments
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    pub(crate) description: Option<Description>,

    /// Default value of this [GraphQL field argument][1] in GraphQL schema.
    ///
    /// If [`None`], then this [argument][1] is a [required][2] one.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Arguments
    /// [2]: https://spec.graphql.org/October2021#sec-Required-Arguments
    pub(crate) default: Option<default::Value>,
}

/// Possible kinds of Rust method arguments for code generation.
#[derive(Debug)]
pub(crate) enum OnMethod {
    /// Regular [GraphQL field argument][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Arguments
    Regular(Box<OnField>),

    /// [`Context`] passed into a [GraphQL field][2] resolving method.
    ///
    /// [`Context`]: juniper::Context
    /// [2]: https://spec.graphql.org/October2021#sec-Language.Fields
    Context(Box<syn::Type>),

    /// [`Executor`] passed into a [GraphQL field][2] resolving method.
    ///
    /// [`Executor`]: juniper::Executor
    /// [2]: https://spec.graphql.org/October2021#sec-Language.Fields
    Executor,
}

impl OnMethod {
    /// Returns this argument as the one [`OnField`], if it represents the one.
    #[must_use]
    pub(crate) fn as_regular(&self) -> Option<&OnField> {
        if let Self::Regular(arg) = self {
            Some(&**arg)
        } else {
            None
        }
    }

    /// Returns [`syn::Type`] of this [`OnMethod::Context`], if it represents
    /// the one.
    #[must_use]
    pub(crate) fn context_ty(&self) -> Option<&syn::Type> {
        if let Self::Context(ty) = self {
            Some(&**ty)
        } else {
            None
        }
    }

    /// Returns generated code for the [`marker::IsOutputType::mark`] method,
    /// which performs static checks for this argument, if it represents an
    /// [`OnField`] one.
    ///
    /// [`marker::IsOutputType::mark`]: juniper::marker::IsOutputType::mark
    #[must_use]
    pub(crate) fn method_mark_tokens(&self, scalar: &scalar::Type) -> Option<TokenStream> {
        let ty = &self.as_regular()?.ty;
        Some(quote_spanned! { ty.span() =>
            <#ty as ::juniper::marker::IsInputType<#scalar>>::mark();
        })
    }

    /// Returns generated code for the [`GraphQLType::meta`] method, which
    /// registers this argument in [`Registry`], if it represents an [`OnField`]
    /// argument.
    ///
    /// [`GraphQLType::meta`]: juniper::GraphQLType::meta
    /// [`Registry`]: juniper::Registry
    #[must_use]
    pub(crate) fn method_meta_tokens(&self) -> Option<TokenStream> {
        let arg = self.as_regular()?;

        let (name, ty) = (&arg.name, &arg.ty);

        let description = &arg.description;

        let method = if let Some(val) = &arg.default {
            quote_spanned! { val.span() =>
                .arg_with_default::<#ty>(#name, &#val, info)
            }
        } else {
            quote! { .arg::<#ty>(#name, info) }
        };

        Some(quote! { .argument(registry #method #description) })
    }

    /// Returns generated code for the [`GraphQLValue::resolve_field`] method,
    /// which provides the value of this [`OnMethod`] argument to be passed into
    /// a trait method call.
    ///
    /// [`GraphQLValue::resolve_field`]: juniper::GraphQLValue::resolve_field
    #[must_use]
    pub(crate) fn method_resolve_field_tokens(
        &self,
        scalar: &scalar::Type,
        for_async: bool,
    ) -> TokenStream {
        match self {
            Self::Regular(arg) => {
                let (name, ty) = (&arg.name, &arg.ty);
                let err_text = format!("Missing argument `{name}`: {{}}");

                let arg = quote! {
                    args.get::<#ty>(#name).and_then(|opt| opt.map_or_else(|| {
                        <#ty as ::juniper::FromInputValue<#scalar>>::from_implicit_null()
                            .map_err(|e| {
                                ::juniper::IntoFieldError::<#scalar>::into_field_error(e)
                                    .map_message(|m| format!(#err_text, m))
                            })
                    }, ::core::result::Result::Ok))
                };
                if for_async {
                    quote! {
                        match #arg {
                            ::core::result::Result::Ok(v) => v,
                            ::core::result::Result::Err(e) => return ::std::boxed::Box::pin(async {
                                ::core::result::Result::Err(e)
                            }),
                        }
                    }
                } else {
                    quote! { #arg? }
                }
            }

            Self::Context(_) => quote! {
                ::juniper::FromContext::from(executor.context())
            },

            Self::Executor => quote! { &executor },
        }
    }

    /// Parses an [`OnMethod`] argument from the given Rust method argument
    /// definition.
    ///
    /// Returns [`None`] if parsing fails and emits parsing errors into the
    /// given `scope`.
    pub(crate) fn parse(
        argument: &mut syn::PatType,
        renaming: &rename::Policy,
        scope: &diagnostic::Scope,
    ) -> Option<Self> {
        let orig_attrs = argument.attrs.clone();

        // Remove repeated attributes from the method, to omit incorrect expansion.
        argument.attrs = mem::take(&mut argument.attrs)
            .into_iter()
            .filter(|attr| !path_eq_single(attr.path(), "graphql"))
            .collect();

        let attr = Attr::from_attrs("graphql", &orig_attrs)
            .map_err(diagnostic::emit_error)
            .ok()?;

        if attr.context.is_some() {
            return Some(Self::Context(Box::new(argument.ty.unreferenced().clone())));
        }
        if attr.executor.is_some() {
            return Some(Self::Executor);
        }
        if let syn::Pat::Ident(name) = &*argument.pat {
            let arg = match name.ident.unraw().to_string().as_str() {
                "context" | "ctx" | "_context" | "_ctx" => {
                    Some(Self::Context(Box::new(argument.ty.unreferenced().clone())))
                }
                "executor" | "_executor" => Some(Self::Executor),
                _ => None,
            };
            if arg.is_some() {
                attr.ensure_no_regular_arguments()
                    .map_err(|e| scope.error(&e).emit())
                    .ok()?;
                return arg;
            }
        }

        let name = if let Some(name) = attr.name.as_ref() {
            name.as_ref().value()
        } else if let syn::Pat::Ident(name) = &*argument.pat {
            renaming.apply(&name.ident.unraw().to_string())
        } else {
            scope
                .custom(
                    argument.pat.span(),
                    "method argument should be declared as a single identifier",
                )
                .note(String::from(
                    "use `#[graphql(name = ...)]` attribute to specify custom argument's \
                     name without requiring it being a single identifier",
                ))
                .emit();
            return None;
        };
        if name.starts_with("__") {
            scope.no_double_underscore(
                attr.name
                    .as_ref()
                    .map(SpanContainer::span_ident)
                    .unwrap_or_else(|| argument.pat.span()),
            );
            return None;
        }

        Some(Self::Regular(Box::new(OnField {
            name,
            ty: argument.ty.as_ref().clone(),
            description: attr.description.map(SpanContainer::into_inner),
            default: attr.default.map(SpanContainer::into_inner),
        })))
    }
}
