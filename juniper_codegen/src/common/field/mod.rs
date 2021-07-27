//! Common functions, definitions and extensions for parsing and code generation
//! of [GraphQL fields][1].

pub(crate) mod arg;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned as _,
    token,
};

use crate::{
    common::{
        gen,
        parse::{
            attr::{err, OptionExt as _},
            ParseBufferExt as _,
        },
        ScalarValueType,
    },
    util::{filter_attrs, get_deprecated, get_doc_comment, span_container::SpanContainer},
};

pub(crate) use self::arg::{OnField as Argument, OnMethod as MethodArgument};

/// Available metadata (arguments) behind `#[graphql]` attribute placed on a
/// [GraphQL field][1] definition.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
#[derive(Debug, Default)]
pub(crate) struct Attr {
    /// Explicitly specified name of this [GraphQL field][1].
    ///
    /// If [`None`], then `camelCased` Rust method name is used by default.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    pub(crate) name: Option<SpanContainer<syn::LitStr>>,

    /// Explicitly specified [description][2] of this [GraphQL field][1].
    ///
    /// If [`None`], then Rust doc comment is used as the [description][2], if
    /// any.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    /// [2]: https://spec.graphql.org/June2018/#sec-Descriptions
    pub(crate) description: Option<SpanContainer<syn::LitStr>>,

    /// Explicitly specified [deprecation][2] of this [GraphQL field][1].
    ///
    /// If [`None`], then Rust `#[deprecated]` attribute is used as the
    /// [deprecation][2], if any.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    /// [2]: https://spec.graphql.org/June2018/#sec-Deprecation
    pub(crate) deprecated: Option<SpanContainer<Option<syn::LitStr>>>,

    /// Explicitly specified marker indicating that this method (or struct
    /// field) should be omitted by code generation and not considered as the
    /// [GraphQL field][1] definition.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    pub(crate) ignore: Option<SpanContainer<syn::Ident>>,

    /// Explicitly specified marker indicating that this trait method doesn't
    /// represent a [GraphQL field][1], but is a downcasting function into the
    /// [GraphQL object][2] implementer type returned by this trait method.
    ///
    /// Once this marker is specified, the [GraphQL object][2] implementer type
    /// cannot be downcast via another trait method or external downcasting
    /// function.
    ///
    /// Omit using this field if you're generating code for [GraphQL object][2].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    /// [2]: https://spec.graphql.org/June2018/#sec-Objects
    pub(crate) downcast: Option<SpanContainer<syn::Ident>>,
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
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
                    let desc = input.parse::<syn::LitStr>()?;
                    out.description
                        .replace(SpanContainer::new(ident.span(), Some(desc.span()), desc))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "deprecated" => {
                    let mut reason = None;
                    if input.is_next::<token::Eq>() {
                        input.parse::<token::Eq>()?;
                        reason = Some(input.parse::<syn::LitStr>()?);
                    }
                    out.deprecated
                        .replace(SpanContainer::new(
                            ident.span(),
                            reason.as_ref().map(|r| r.span()),
                            reason,
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "ignore" | "skip" => out
                    .ignore
                    .replace(SpanContainer::new(ident.span(), None, ident.clone()))
                    .none_or_else(|_| err::dup_arg(&ident))?,
                "downcast" => out
                    .downcast
                    .replace(SpanContainer::new(ident.span(), None, ident.clone()))
                    .none_or_else(|_| err::dup_arg(&ident))?,
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
    /// Tries to merge two [`Attrs`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            deprecated: try_merge_opt!(deprecated: self, another),
            ignore: try_merge_opt!(ignore: self, another),
            downcast: try_merge_opt!(downcast: self, another),
        })
    }

    /// Parses [`Attr`] from the given multiple `name`d [`syn::Attribute`]s
    /// placed on a [GraphQL field][1] definition.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    pub(crate) fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut attr = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if let Some(ignore) = &attr.ignore {
            if attr.name.is_some()
                || attr.description.is_some()
                || attr.deprecated.is_some()
                || attr.downcast.is_some()
            {
                return Err(syn::Error::new(
                    ignore.span(),
                    "`ignore` attribute argument is not composable with any other arguments",
                ));
            }
        }

        if let Some(downcast) = &attr.downcast {
            if attr.name.is_some()
                || attr.description.is_some()
                || attr.deprecated.is_some()
                || attr.ignore.is_some()
            {
                return Err(syn::Error::new(
                    downcast.span(),
                    "`downcast` attribute argument is not composable with any other arguments",
                ));
            }
        }

        if attr.description.is_none() {
            attr.description = get_doc_comment(attrs).map(|sc| {
                let span = sc.span_ident();
                sc.map(|desc| syn::LitStr::new(&desc, span))
            });
        }

        if attr.deprecated.is_none() {
            attr.deprecated = get_deprecated(attrs).map(|sc| {
                let span = sc.span_ident();
                sc.map(|depr| depr.reason.map(|rsn| syn::LitStr::new(&rsn, span)))
            });
        }

        Ok(attr)
    }
}

/// Representation of a [GraphQL field][1] for code generation.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
#[derive(Debug)]
pub(crate) struct Definition {
    /// Rust type that this [GraphQL field][1] is represented by (method return
    /// type or struct field type).
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    pub(crate) ty: syn::Type,

    /// Name of this [GraphQL field][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    pub(crate) name: String,

    /// [Description][2] of this [GraphQL field][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    /// [2]: https://spec.graphql.org/June2018/#sec-Descriptions
    pub(crate) description: Option<String>,

    /// [Deprecation][2] of this [GraphQL field][1] to put into GraphQL schema.
    ///
    /// If inner [`Option`] is [`None`], then deprecation has no message
    /// attached.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    /// [2]: https://spec.graphql.org/June2018/#sec-Deprecation
    pub(crate) deprecated: Option<Option<String>>,

    /// Ident of the Rust method (or struct field) representing this
    /// [GraphQL field][1].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    pub(crate) ident: syn::Ident,

    /// Rust [`MethodArgument`]s required to call the method representing this
    /// [GraphQL field][1].
    ///
    /// If [`None`] then this [GraphQL field][1] is represented by a struct
    /// field.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    pub(crate) arguments: Option<Vec<MethodArgument>>,

    /// [`syn::Receiver`] of the Rust method representing this
    /// [GraphQL field][1].
    ///
    /// If [`None`] then this method has no receiver.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    pub(crate) receiver: Option<syn::Receiver>,

    /// Indicator whether this [GraphQL field][1] should be resolved
    /// asynchronously.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    pub(crate) is_async: bool,
}

impl Definition {
    /// Indicates whether this [GraphQL field][1] is represented by a method,
    /// not a struct field.
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    #[must_use]
    pub(crate) fn is_method(&self) -> bool {
        self.arguments.is_none()
    }

    /// Returns generated code that panics about unknown [GraphQL field][1]
    /// tried to be resolved in the [`GraphQLValue::resolve_field`] method.
    ///
    /// [`GraphQLValue::resolve_field`]: juniper::GraphQLValue::resolve_field
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    #[must_use]
    pub(crate) fn method_resolve_field_panic_no_field_tokens(
        scalar: &ScalarValueType,
    ) -> TokenStream {
        quote! {
            panic!(
                "Field `{}` not found on type `{}`",
                field,
                <Self as ::juniper::GraphQLType<#scalar>>::name(info).unwrap(),
            )
        }
    }

    /// Returns generated code that panics about [GraphQL fields][1] tried to be
    /// resolved asynchronously in the [`GraphQLValue::resolve_field`] method
    /// (which is synchronous itself).
    ///
    /// [`GraphQLValue::resolve_field`]: juniper::GraphQLValue::resolve_field
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    #[must_use]
    pub(crate) fn method_resolve_field_panic_async_field_tokens(
        field_names: &[&str],
        scalar: &ScalarValueType,
    ) -> TokenStream {
        quote! {
            #( #field_names )|* => panic!(
                "Tried to resolve async field `{}` on type `{}` with a sync resolver",
                field,
                <Self as ::juniper::GraphQLType<#scalar>>::name(info).unwrap(),
            ),
        }
    }

    /// Returns generated code for the [`marker::IsOutputType::mark`] method,
    /// which performs static checks for this [GraphQL field][1].
    ///
    /// [`marker::IsOutputType::mark`]: juniper::marker::IsOutputType::mark
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    #[must_use]
    pub(crate) fn method_mark_tokens(&self, scalar: &ScalarValueType) -> TokenStream {
        let args = self.arguments.unwrap_or_default();
        let args_marks = args.iter().filter_map(|a| a.method_mark_tokens(scalar));

        let ty = &self.ty;
        let resolved_ty = quote! {
            <#ty as ::juniper::IntoResolvable<
                '_, #scalar, _, <Self as ::juniper::GraphQLValue<#scalar>>::Context,
            >>::Type
        };

        quote! {
            #( #args_marks )*
            <#resolved_ty as ::juniper::marker::IsOutputType<#scalar>>::mark();
        }
    }

    /// Returns generated code for the [`GraphQLType::meta`] method, which
    /// registers this [GraphQL field][1] in [`Registry`].
    ///
    /// [`GraphQLType::meta`]: juniper::GraphQLType::meta
    /// [`Registry`]: juniper::Registry
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    #[must_use]
    pub(crate) fn method_meta_tokens(&self) -> TokenStream {
        let (name, ty) = (&self.name, &self.ty);

        let description = self
            .description
            .as_ref()
            .map(|desc| quote! { .description(#desc) });

        let deprecated = self.deprecated.as_ref().map(|reason| {
            let reason = reason
                .as_ref()
                .map(|rsn| quote! { Some(#rsn) })
                .unwrap_or_else(|| quote! { None });
            quote! { .deprecated(#reason) }
        });

        let args = self
            .arguments
            .iter()
            .filter_map(MethodArgument::method_meta_tokens);

        quote! {
            registry.field_convert::<#ty, _, Self::Context>(#name, info)
                #( #args )*
                #description
                #deprecated
        }
    }

    /// Returns generated code for the [`GraphQLValue::resolve_field`] method,
    /// which resolves this [GraphQL field][1] synchronously.
    ///
    /// Returns [`None`] if this [`Definition::is_async`].
    ///
    /// [`GraphQLValue::resolve_field`]: juniper::GraphQLValue::resolve_field
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    #[must_use]
    pub(crate) fn method_resolve_field_tokens(
        &self,
        trait_ty: Option<&syn::Type>,
    ) -> Option<TokenStream> {
        if self.is_async {
            return None;
        }

        let (name, ty, ident) = (&self.name, &self.ty, &self.ident);

        let res = if self.is_method() {
            let args = self
                .arguments
                .as_ref()
                .unwrap()
                .iter()
                .map(MethodArgument::method_resolve_field_tokens);

            let rcv = self.receiver.is_some().then(|| {
                quote! { self, }
            });

            if trait_ty.is_some() {
                quote! { <Self as #trait_ty>::#ident(#rcv #( #args ),*) }
            } else {
                quote! { Self::#ident(#rcv #( #args ),*) }
            }
        } else {
            quote! { self.#ident }
        };

        let resolving_code = gen::sync_resolving_code();

        Some(quote! {
            #name => {
                let res: #ty = #res;
                #resolving_code
            }
        })
    }

    /// Returns generated code for the
    /// [`GraphQLValueAsync::resolve_field_async`] method, which resolves this
    /// [GraphQL field][1] asynchronously.
    ///
    /// [`GraphQLValueAsync::resolve_field_async`]: juniper::GraphQLValueAsync::resolve_field_async
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    #[must_use]
    pub(crate) fn method_resolve_field_async_tokens(
        &self,
        trait_ty: Option<&syn::Type>,
    ) -> TokenStream {
        let (name, ty, ident) = (&self.name, &self.ty, &self.ident);

        let mut fut = if self.is_method() {
            let args = self
                .arguments
                .as_ref()
                .unwrap()
                .iter()
                .map(MethodArgument::method_resolve_field_tokens);

            let rcv = self.receiver.is_some().then(|| {
                quote! { self, }
            });

            if trait_ty.is_some() {
                quote! { <Self as #trait_ty>::#ident(#rcv #( #args ),*) }
            } else {
                quote! { Self::#ident(#rcv #( #args ),*) }
            }
        } else {
            quote! { self.#ident }
        };
        if !self.is_async {
            fut = quote! { ::juniper::futures::future::ready(#fut) };
        }

        let resolving_code = gen::async_resolving_code(Some(ty));

        quote! {
            #name => {
                let fut = #fut;
                #resolving_code
            }
        }
    }
}
