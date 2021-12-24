//! Common functions, definitions and extensions for parsing and code generation
//! of [GraphQL fields][1]
//!
//! [1]: https://spec.graphql.org/June2018/#sec-Language.Fields.

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
        scalar,
    },
    util::{filter_attrs, get_deprecated, get_doc_comment, span_container::SpanContainer},
};

pub(crate) use self::arg::OnMethod as MethodArgument;

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

    /// Indicator whether the Rust method representing this [GraphQL field][1]
    /// has a [`syn::Receiver`].
    ///
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    pub(crate) has_receiver: bool,

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
        self.arguments.is_some()
    }

    /// Returns generated code that errors about unknown [GraphQL field][1]
    /// tried to be resolved in the [`GraphQLValue::resolve_field`] method.
    ///
    /// [`GraphQLValue::resolve_field`]: juniper::GraphQLValue::resolve_field
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    #[must_use]
    pub(crate) fn method_resolve_field_err_no_field_tokens(
        scalar: &scalar::Type,
        ty_name: &str,
    ) -> TokenStream {
        quote! {
            return Err(::juniper::FieldError::from(format!(
                "Field `{}` not found on type `{}`",
                field,
                <Self as ::juniper::GraphQLType<#scalar>>::name(info)
                    .ok_or_else(|| ::juniper::macros::helper::err_unnamed_type(#ty_name))?,
            )))
        }
    }

    /// Returns generated code that errors about [GraphQL fields][1] tried to be
    /// resolved asynchronously in the [`GraphQLValue::resolve_field`] method
    /// (which is synchronous itself).
    ///
    /// [`GraphQLValue::resolve_field`]: juniper::GraphQLValue::resolve_field
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    #[must_use]
    pub(crate) fn method_resolve_field_err_async_field_tokens(
        field_names: &[&str],
        scalar: &scalar::Type,
        ty_name: &str,
    ) -> TokenStream {
        quote! {
            #( #field_names )|* => return Err(::juniper::FieldError::from(format!(
                "Tried to resolve async field `{}` on type `{}` with a sync resolver",
                field,
                <Self as ::juniper::GraphQLType<#scalar>>::name(info)
                    .ok_or_else(|| ::juniper::macros::helper::err_unnamed_type(#ty_name))?,
            ))),
        }
    }

    /// Returns generated code for the [`marker::IsOutputType::mark`] method,
    /// which performs static checks for this [GraphQL field][1].
    ///
    /// [`marker::IsOutputType::mark`]: juniper::marker::IsOutputType::mark
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
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
            .flat_map(|args| args.iter().filter_map(MethodArgument::method_meta_tokens));

        quote! {
            registry.field_convert::<#ty, _, Self::Context>(#name, info)
                #( #args )*
                #description
                #deprecated
        }
    }

    /// Returns generated code for the [`GraphQLValue::resolve_field`][0]
    /// method, which resolves this [GraphQL field][1] synchronously.
    ///
    /// Returns [`None`] if this [`Definition::is_async`].
    ///
    /// [0]: juniper::GraphQLValue::resolve_field
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    #[must_use]
    pub(crate) fn method_resolve_field_tokens(
        &self,
        scalar: &scalar::Type,
        trait_ty: Option<&syn::Type>,
    ) -> Option<TokenStream> {
        if self.is_async {
            return None;
        }

        let (name, mut ty, ident) = (&self.name, self.ty.clone(), &self.ident);

        let res = if self.is_method() {
            let args = self
                .arguments
                .as_ref()
                .unwrap()
                .iter()
                .map(|arg| arg.method_resolve_field_tokens(scalar, false));

            let rcv = self.has_receiver.then(|| {
                quote! { self, }
            });

            if trait_ty.is_some() {
                quote! { <Self as #trait_ty>::#ident(#rcv #( #args ),*) }
            } else {
                quote! { Self::#ident(#rcv #( #args ),*) }
            }
        } else {
            ty = parse_quote! { _ };
            quote! { &self.#ident }
        };

        let resolving_code = gen::sync_resolving_code();

        Some(quote! {
            #name => {
                let res: #ty = #res;
                #resolving_code
            }
        })
    }

    /// TODO
    #[must_use]
    pub(crate) fn impl_field(
        &self,
        impl_ty: &syn::Type,
        impl_generics: &TokenStream,
        where_clause: Option<&syn::WhereClause>,
        scalar: &scalar::Type,
        trait_ty: Option<&syn::Type>,
        context: &syn::Type,
    ) -> Option<TokenStream> {
        if self.is_async {
            return None;
        }

        let (name, ty, mut res_ty, ident) =
            (&self.name, self.ty.clone(), self.ty.clone(), &self.ident);

        let res = if self.is_method() {
            let args = self
                .arguments
                .as_ref()
                .unwrap()
                .iter()
                .map(|arg| arg.method_resolve_field_tokens(scalar, false));

            let rcv = self.has_receiver.then(|| {
                quote! { self, }
            });

            if trait_ty.is_some() {
                quote! { <Self as #trait_ty>::#ident(#rcv #( #args ),*) }
            } else {
                quote! { Self::#ident(#rcv #( #args ),*) }
            }
        } else {
            res_ty = parse_quote! { _ };
            quote! { &self.#ident }
        };

        let arguments = self
            .arguments
            .as_ref()
            .iter()
            .flat_map(|vec| vec.iter())
            .filter_map(|arg| match arg {
                MethodArgument::Regular(arg) => {
                    let (name, ty) = (&arg.name, &arg.ty);

                    Some(quote! {(
                        #name,
                        <#ty as ::juniper::macros::helper::BaseType<#scalar>>::NAME,
                        <#ty as ::juniper::macros::helper::WrappedType<#scalar>>::VALUE,
                    )})
                }
                MethodArgument::Executor | MethodArgument::Context(_) => None,
            })
            .collect::<Vec<_>>();

        let resolving_code = gen::sync_resolving_code();

        Some(quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::macros::helper::Field<
                #scalar,
                { ::juniper::macros::helper::fnv1a128(#name) }
            > for #impl_ty
                #where_clause
            {
                type Context = #context;
                type TypeInfo = ();
                const TYPE: ::juniper::macros::helper::Type =
                    <#ty as ::juniper::macros::helper::BaseType<#scalar>>::NAME;
                const SUB_TYPES: ::juniper::macros::helper::Types =
                    <#ty as ::juniper::macros::helper::BaseSubTypes<#scalar>>::NAMES;
                const WRAPPED_VALUE: juniper::macros::helper::WrappedValue =
                    <#ty as ::juniper::macros::helper::WrappedType<#scalar>>::VALUE;
                const ARGUMENTS: &'static [(
                    ::juniper::macros::helper::Name,
                    ::juniper::macros::helper::Type,
                    ::juniper::macros::helper::WrappedValue,
                )] = &[#(#arguments,)*];

                fn call(
                    &self,
                    info: &Self::TypeInfo,
                    args: &::juniper::Arguments<#scalar>,
                    executor: &::juniper::Executor<Self::Context, #scalar>,
                ) -> ::juniper::ExecutionResult<#scalar> {
                    let res: #res_ty = #res;
                    #resolving_code
                }
            }
        })
    }

    /// Returns generated code for the
    /// [`GraphQLValueAsync::resolve_field_async`][0] method, which resolves
    /// this [GraphQL field][1] asynchronously.
    ///
    /// [0]: juniper::GraphQLValueAsync::resolve_field_async
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    #[must_use]
    pub(crate) fn method_resolve_field_async_tokens(
        &self,
        scalar: &scalar::Type,
        trait_ty: Option<&syn::Type>,
    ) -> TokenStream {
        let (name, mut ty, ident) = (&self.name, self.ty.clone(), &self.ident);

        let mut fut = if self.is_method() {
            let args = self
                .arguments
                .as_ref()
                .unwrap()
                .iter()
                .map(|arg| arg.method_resolve_field_tokens(scalar, true));

            let rcv = self.has_receiver.then(|| {
                quote! { self, }
            });

            if trait_ty.is_some() {
                quote! { <Self as #trait_ty>::#ident(#rcv #( #args ),*) }
            } else {
                quote! { Self::#ident(#rcv #( #args ),*) }
            }
        } else {
            ty = parse_quote! { _ };
            quote! { &self.#ident }
        };
        if !self.is_async {
            fut = quote! { ::juniper::futures::future::ready(#fut) };
        }

        let resolving_code = gen::async_resolving_code(Some(&ty));

        quote! {
            #name => {
                let fut = #fut;
                #resolving_code
            }
        }
    }

    /// Returns generated code for the
    /// [`GraphQLSubscriptionValue::resolve_field_into_stream`][0] method, which
    /// resolves this [GraphQL field][1] as [subscription][2].
    ///
    /// [0]: juniper::GraphQLSubscriptionValue::resolve_field_into_stream
    /// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
    /// [2]: https://spec.graphql.org/June2018/#sec-Subscription
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
                            ::juniper::IntoResolvable::into(res, executor.context());
                        async move {
                            let ex = executor.as_executor();
                            match res2 {
                                Ok(Some((ctx, r))) => {
                                    let sub = ex.replaced_context(ctx);
                                    sub.resolve_with_ctx_async(&(), &r)
                                        .await
                                        .map_err(|e| ex.new_error(e))
                                }
                                Ok(None) => Ok(::juniper::Value::null()),
                                Err(e) => Err(ex.new_error(e)),
                            }
                        }
                    });
                    Ok(::juniper::Value::Scalar::<
                        ::juniper::ValuesStream::<#scalar>
                    >(::juniper::futures::StreamExt::boxed(stream)))
                })
            }
        }
    }
}

/// Checks whether all [GraphQL fields][1] fields have different names.
///
/// [1]: https://spec.graphql.org/June2018/#sec-Language.Fields
#[must_use]
pub(crate) fn all_different(fields: &[Definition]) -> bool {
    let mut names: Vec<_> = fields.iter().map(|f| &f.name).collect();
    names.dedup();
    names.len() == fields.len()
}
