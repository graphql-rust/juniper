//! Common functions, definitions and extensions for parsing and code generation
//! of [GraphQL fields][1]
//!
//! [1]: https://spec.graphql.org/October2021#sec-Language.Fields

pub(crate) mod arg;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned as _,
    token,
};

use crate::common::{
    deprecation, filter_attrs,
    parse::{
        attr::{err, OptionExt as _},
        ParseBufferExt as _,
    },
    scalar, Description, SpanContainer,
};

pub(crate) use self::arg::OnMethod as MethodArgument;

/// Available metadata (arguments) behind `#[graphql]` attribute placed on a
/// [GraphQL field][1] definition.
///
/// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
#[derive(Debug, Default)]
pub(crate) struct Attr {
    /// Explicitly specified name of this [GraphQL field][1].
    ///
    /// If [`None`], then `camelCased` Rust method name is used by default.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    pub(crate) name: Option<SpanContainer<syn::LitStr>>,

    /// Explicitly specified [description][2] of this [GraphQL field][1].
    ///
    /// If [`None`], then Rust doc comment will be used as the [description][2],
    /// if any.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    pub(crate) description: Option<SpanContainer<Description>>,

    /// Explicitly specified [deprecation][2] of this [GraphQL field][1].
    ///
    /// If [`None`], then Rust `#[deprecated]` attribute will be used as the
    /// [deprecation][2], if any.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    /// [2]: https://spec.graphql.org/October2021#sec-Deprecation
    pub(crate) deprecated: Option<SpanContainer<deprecation::Directive>>,

    /// Explicitly specified marker indicating that this method (or struct
    /// field) should be omitted by code generation and not considered as the
    /// [GraphQL field][1] definition.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    pub(crate) ignore: Option<SpanContainer<syn::Ident>>,
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
                "deprecated" => {
                    let directive = input.parse::<deprecation::Directive>()?;
                    out.deprecated
                        .replace(SpanContainer::new(
                            ident.span(),
                            directive.reason.as_ref().map(|r| r.span()),
                            directive,
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "ignore" | "skip" => out
                    .ignore
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
        })
    }

    /// Parses [`Attr`] from the given multiple `name`d [`syn::Attribute`]s
    /// placed on a [GraphQL field][1] definition.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    pub(crate) fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut attr = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if let Some(ignore) = &attr.ignore {
            if attr.name.is_some() || attr.description.is_some() || attr.deprecated.is_some() {
                return Err(syn::Error::new(
                    ignore.span(),
                    "`ignore` attribute argument is not composable with any other arguments",
                ));
            }
        }

        if attr.description.is_none() {
            attr.description = Description::parse_from_doc_attrs(attrs)?;
        }

        if attr.deprecated.is_none() {
            attr.deprecated = deprecation::Directive::parse_from_deprecated_attr(attrs)?;
        }

        Ok(attr)
    }
}

/// Representation of a [GraphQL field][1] for code generation.
///
/// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
#[derive(Debug)]
pub(crate) struct Definition {
    /// Rust type that this [GraphQL field][1] is represented by (method return
    /// type or struct field type).
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    pub(crate) ty: syn::Type,

    /// Name of this [GraphQL field][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    pub(crate) name: String,

    /// [Description][2] of this [GraphQL field][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    pub(crate) description: Option<Description>,

    /// [Deprecation][2] of this [GraphQL field][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    /// [2]: https://spec.graphql.org/October2021#sec-Deprecation
    pub(crate) deprecated: Option<deprecation::Directive>,

    /// Ident of the Rust method (or struct field) representing this
    /// [GraphQL field][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    pub(crate) ident: syn::Ident,

    /// Rust [`MethodArgument`]s required to call the method representing this
    /// [GraphQL field][1].
    ///
    /// If [`None`] then this [GraphQL field][1] is represented by a struct
    /// field.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    pub(crate) arguments: Option<Vec<MethodArgument>>,

    /// Indicator whether the Rust method representing this [GraphQL field][1]
    /// has a [`syn::Receiver`].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    pub(crate) has_receiver: bool,

    /// Indicator whether this [GraphQL field][1] should be resolved
    /// asynchronously.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    pub(crate) is_async: bool,
}

impl Definition {
    /// Indicates whether this [GraphQL field][1] is represented by a method,
    /// not a struct field.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    #[must_use]
    pub(crate) fn is_method(&self) -> bool {
        self.arguments.is_some()
    }

    /// Returns generated code that errors about unknown [GraphQL field][1]
    /// tried to be resolved in the [`GraphQLValue::resolve_field`] method.
    ///
    /// [`GraphQLValue::resolve_field`]: juniper::GraphQLValue::resolve_field
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    #[must_use]
    pub(crate) fn method_resolve_field_err_no_field_tokens(
        scalar: &scalar::Type,
        ty_name: &str,
    ) -> TokenStream {
        quote! {
            return ::core::result::Result::Err(::juniper::FieldError::from(::std::format!(
                "Field `{}` not found on type `{}`",
                field,
                <Self as ::juniper::GraphQLType<#scalar>>::name(info)
                    .ok_or_else(|| ::juniper::macros::helper::err_unnamed_type(#ty_name))?,
            )))
        }
    }

    /// Returns generated code for the [`marker::IsOutputType::mark`] method,
    /// which performs static checks for this [GraphQL field][1].
    ///
    /// [`marker::IsOutputType::mark`]: juniper::marker::IsOutputType::mark
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    #[must_use]
    pub(crate) fn method_mark_tokens(
        &self,
        infer_result: bool,
        scalar: &scalar::Type,
    ) -> TokenStream {
        let args_marks = self
            .arguments
            .iter()
            .flat_map(|args| args.iter().filter_map(|a| a.method_mark_tokens(scalar)));

        let ty = &self.ty;
        let mut ty = quote! { #ty };
        if infer_result {
            ty = quote! {
                <#ty as ::juniper::IntoFieldResult::<_, #scalar>>::Item
            };
        }
        let resolved_ty = quote! {
            <#ty as ::juniper::IntoResolvable<
                '_, #scalar, _, <Self as ::juniper::GraphQLValue<#scalar>>::Context,
            >>::Type
        };

        quote_spanned! { self.ty.span() =>
            #( #args_marks )*
            <#resolved_ty as ::juniper::marker::IsOutputType<#scalar>>::mark();
        }
    }

    /// Returns generated code for the [`GraphQLType::meta`] method, which
    /// registers this [GraphQL field][1] in [`Registry`].
    ///
    /// [`GraphQLType::meta`]: juniper::GraphQLType::meta
    /// [`Registry`]: juniper::Registry
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    #[must_use]
    pub(crate) fn method_meta_tokens(
        &self,
        extract_stream_type: Option<&scalar::Type>,
    ) -> TokenStream {
        let (name, ty) = (&self.name, &self.ty);
        let mut ty = quote! { #ty };
        if let Some(scalar) = extract_stream_type {
            ty = quote! {
                <#ty as ::juniper::ExtractTypeFromStream<_, #scalar>>::Item
            };
        }

        let description = &self.description;
        let deprecated = &self.deprecated;

        let args = self
            .arguments
            .iter()
            .flat_map(|args| args.iter().filter_map(MethodArgument::method_meta_tokens));

        quote! {
            registry.field_convert::<#ty, _, Self::Context>(#name, info)
                #( #args )*
                #description
                #deprecated
        }
    }

    /// Returns generated code for the
    /// [`GraphQLSubscriptionValue::resolve_field_into_stream`][0] method, which
    /// resolves this [GraphQL field][1] as [subscription][2].
    ///
    /// [0]: juniper::GraphQLSubscriptionValue::resolve_field_into_stream
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    /// [2]: https://spec.graphql.org/October2021#sec-Subscription
    #[must_use]
    pub(crate) fn method_resolve_field_into_stream_tokens(
        &self,
        scalar: &scalar::Type,
    ) -> TokenStream {
        let (name, mut ty, ident) = (&self.name, self.ty.clone(), &self.ident);

        let mut fut = if self.is_method() {
            let args = self
                .arguments
                .as_ref()
                .unwrap()
                .iter()
                .map(|arg| arg.method_resolve_field_tokens(scalar, false));

            let rcv = self.has_receiver.then(|| {
                quote! { self, }
            });

            quote! { Self::#ident(#rcv #( #args ),*) }
        } else {
            ty = parse_quote! { _ };
            quote! { &self.#ident }
        };
        if !self.is_async {
            fut = quote! { ::juniper::futures::future::ready(#fut) };
        }

        quote! {
            #name => {
                ::juniper::futures::FutureExt::boxed(async move {
                    let res: #ty = #fut.await;
                    let res = ::juniper::IntoFieldResult::<_, #scalar>::into_result(res)?;
                    let executor = executor.as_owned_executor();
                    let stream = ::juniper::futures::StreamExt::then(res, move |res| {
                        let executor = executor.clone();
                        let res2: ::juniper::FieldResult<_, #scalar> =
                            ::juniper::IntoResolvable::into_resolvable(res, executor.context());
                        async move {
                            let ex = executor.as_executor();
                            match res2 {
                                ::core::result::Result::Ok(
                                    ::core::option::Option::Some((ctx, r)),
                                ) => {
                                    let sub = ex.replaced_context(ctx);
                                    sub.resolve_with_ctx_async(&(), &r)
                                        .await
                                        .map_err(|e| ex.new_error(e))
                                }
                                ::core::result::Result::Ok(::core::option::Option::None) => {
                                    ::core::result::Result::Ok(::juniper::Value::null())
                                }
                                ::core::result::Result::Err(e) => {
                                    ::core::result::Result::Err(ex.new_error(e))
                                }
                            }
                        }
                    });
                    ::core::result::Result::Ok(::juniper::Value::Scalar::<
                        ::juniper::ValuesStream::<#scalar>
                    >(::juniper::futures::StreamExt::boxed(stream)))
                })
            }
        }
    }
}

/// Checks whether all [GraphQL fields][1] fields have different names.
///
/// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
#[must_use]
pub(crate) fn all_different(fields: &[Definition]) -> bool {
    let mut names: Vec<_> = fields.iter().map(|f| &f.name).collect();
    names.dedup();
    names.len() == fields.len()
}
