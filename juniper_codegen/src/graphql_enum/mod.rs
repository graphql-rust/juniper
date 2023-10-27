//! Code generation for [GraphQL enums][0].
//!
//! [0]: https://spec.graphql.org/October2021#sec-Enums

pub(crate) mod derive;

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    ext::IdentExt as _,
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
    rename, scalar, Description, SpanContainer,
};

/// Available arguments behind `#[graphql]` attribute placed on a Rust enum
/// definition, when generating code for a [GraphQL enum][0].
///
/// [0]: https://spec.graphql.org/October2021#sec-Enums
#[derive(Debug, Default)]
struct ContainerAttr {
    /// Explicitly specified name of this [GraphQL enum][0].
    ///
    /// If [`None`], then Rust enum name will be used by default.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    name: Option<SpanContainer<String>>,

    /// Explicitly specified [description][2] of this [GraphQL enum][0].
    ///
    /// If [`None`], then Rust doc comment will be used as the [description][2],
    /// if any.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    description: Option<SpanContainer<Description>>,

    /// Explicitly specified type of [`Context`] to use for resolving this
    /// [GraphQL enum][0] type with.
    ///
    /// If [`None`], then unit type `()` is assumed as a type of [`Context`].
    ///
    /// [`Context`]: juniper::Context
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    context: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified type (or type parameter with its bounds) of
    /// [`ScalarValue`] to resolve this [GraphQL enum][0] type with.
    ///
    /// If [`None`], then generated code will be generic over any
    /// [`ScalarValue`] type.
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    scalar: Option<SpanContainer<scalar::AttrValue>>,

    /// Explicitly specified [`rename::Policy`] for all [values][1] of this
    /// [GraphQL enum][0].
    ///
    /// If [`None`], then the [`rename::Policy::ScreamingSnakeCase`] will be
    /// applied by default.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    /// [1]: https://spec.graphql.org/October2021#EnumValuesDefinition
    rename_values: Option<SpanContainer<rename::Policy>>,

    /// Indicator whether the generated code is intended to be used only inside
    /// the [`juniper`] library.
    is_internal: bool,
}

impl Parse for ContainerAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut out = Self::default();
        while !input.is_empty() {
            let ident = input.parse_any_ident()?;
            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<token::Eq>()?;
                    let name = input.parse::<syn::LitStr>()?;
                    out.name
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(name.span()),
                            name.value(),
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "desc" | "description" => {
                    input.parse::<token::Eq>()?;
                    let desc = input.parse::<Description>()?;
                    out.description
                        .replace(SpanContainer::new(ident.span(), Some(desc.span()), desc))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "ctx" | "context" | "Context" => {
                    input.parse::<token::Eq>()?;
                    let ctx = input.parse::<syn::Type>()?;
                    out.context
                        .replace(SpanContainer::new(ident.span(), Some(ctx.span()), ctx))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "scalar" | "Scalar" | "ScalarValue" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<scalar::AttrValue>()?;
                    out.scalar
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "rename_all" => {
                    input.parse::<token::Eq>()?;
                    let val = input.parse::<syn::LitStr>()?;
                    out.rename_values
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(val.span()),
                            val.try_into()?,
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?;
                }
                "internal" => {
                    out.is_internal = true;
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

impl ContainerAttr {
    /// Tries to merge two [`ContainerAttr`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            context: try_merge_opt!(context: self, another),
            scalar: try_merge_opt!(scalar: self, another),
            rename_values: try_merge_opt!(rename_values: self, another),
            is_internal: self.is_internal || another.is_internal,
        })
    }

    /// Parses [`ContainerAttr`] from the given multiple `name`d
    /// [`syn::Attribute`]s placed on a trait definition.
    fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut attr = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if attr.description.is_none() {
            attr.description = Description::parse_from_doc_attrs(attrs)?;
        }

        Ok(attr)
    }
}

/// Available arguments behind `#[graphql]` attribute when generating code for
/// a [GraphQL enum][0]'s [value][1].
///
/// [0]: https://spec.graphql.org/October2021#sec-Enums
/// [1]: https://spec.graphql.org/October2021#sec-Enum-Value
#[derive(Debug, Default)]
struct VariantAttr {
    /// Explicitly specified name of this [GraphQL enum value][1].
    ///
    /// If [`None`], then Rust enum variant's name will be used by default.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Enum-Value
    name: Option<SpanContainer<String>>,

    /// Explicitly specified [description][2] of this [GraphQL enum value][1].
    ///
    /// If [`None`], then Rust doc comment will be used as the [description][2],
    /// if any.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Enum-Value
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    description: Option<SpanContainer<Description>>,

    /// Explicitly specified [deprecation][2] of this [GraphQL enum value][1].
    ///
    /// If [`None`], then Rust `#[deprecated]` attribute will be used as the
    /// [deprecation][2], if any.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Enum-Value
    /// [2]: https://spec.graphql.org/October2021#sec--deprecated
    /// [3]: https://spec.graphql.org/October2021#sel-GAHnBZDACEDDGAA_6L
    deprecated: Option<SpanContainer<deprecation::Directive>>,

    /// Explicitly specified marker for the Rust enum variant to be ignored and
    /// not included into the code generated for a [GraphQL enum][0]
    /// implementation.
    ///
    /// [0]: https://spec.graphql.org/October20210#sec-Enums
    ignore: Option<SpanContainer<syn::Ident>>,
}

impl Parse for VariantAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut out = Self::default();
        while !input.is_empty() {
            let ident = input.parse_any_ident()?;
            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<token::Eq>()?;
                    let name = input.parse::<syn::LitStr>()?;
                    out.name
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(name.span()),
                            name.value(),
                        ))
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

impl VariantAttr {
    /// Tries to merge two [`VariantAttr`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            deprecated: try_merge_opt!(deprecated: self, another),
            ignore: try_merge_opt!(ignore: self, another),
        })
    }

    /// Parses [`VariantAttr`] from the given multiple `name`d
    /// [`syn::Attribute`]s placed on a trait definition.
    fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut attr = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if attr.description.is_none() {
            attr.description = Description::parse_from_doc_attrs(attrs)?;
        }

        if attr.deprecated.is_none() {
            attr.deprecated = deprecation::Directive::parse_from_deprecated_attr(attrs)?;
        }

        Ok(attr)
    }
}

/// Representation of a [GraphQL enum value][1] for code generation.
///
/// [1]: https://spec.graphql.org/October2021#sec-Enum-Value
#[derive(Debug)]
struct ValueDefinition {
    /// [`Ident`] of the Rust enum variant behind this [GraphQL enum value][1].
    ///
    /// [`Ident`]: syn::Ident
    /// [1]: https://spec.graphql.org/October2021#sec-Enum-Value
    ident: syn::Ident,

    /// Name of this [GraphQL enum value][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Enum-Value
    name: Box<str>,

    /// [Description][2] of this [GraphQL enum value][1] to put into GraphQL
    /// schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Enum-Value
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    description: Option<Description>,

    /// [Deprecation][2] of this [GraphQL enum value][1] to put into GraphQL
    /// schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Enum-Value
    /// [2]: https://spec.graphql.org/October2021#sec--deprecated
    deprecated: Option<deprecation::Directive>,
}

/// Representation of a [GraphQL enum][0] for code generation.
///
/// [0]: https://spec.graphql.org/October2021#sec-Enums
struct Definition {
    /// [`Ident`] of the Rust enum behind this [GraphQL enum][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    ident: syn::Ident,

    /// [`Generics`] of the Rust enum behind this [GraphQL enum][0].
    ///
    /// [`Generics`]: syn::Generics
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    generics: syn::Generics,

    /// Name of this [GraphQL enum][0] in GraphQL schema.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    name: Box<str>,

    /// [Description][2] of this [GraphQL enum][0] to put into GraphQL schema.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    description: Option<Description>,

    /// Rust type of [`Context`] to generate [`GraphQLType`] implementation with
    /// for this [GraphQL enum][0].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`Context`]: juniper::Context
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    context: syn::Type,

    /// [`ScalarValue`] parametrization to generate [`GraphQLType`]
    /// implementation with for this [GraphQL enum][0].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    scalar: scalar::Type,

    /// [Values][1] of this [GraphQL enum][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    /// [1]: https://spec.graphql.org/October2021#EnumValuesDefinition
    values: Vec<ValueDefinition>,

    /// Indicates whether the Rust enum behind this [GraphQL enum][0] contains
    /// ignored variants.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    has_ignored_variants: bool,
}

impl ToTokens for Definition {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.impl_input_and_output_type_tokens().to_tokens(into);
        self.impl_graphql_type_tokens().to_tokens(into);
        self.impl_graphql_value_tokens().to_tokens(into);
        self.impl_graphql_value_async_tokens().to_tokens(into);
        self.impl_from_input_value_tokens().to_tokens(into);
        self.impl_to_input_value_tokens().to_tokens(into);
        self.impl_reflection_traits_tokens().to_tokens(into);
    }
}

impl Definition {
    /// Returns generated code implementing [`marker::IsOutputType`] trait for
    /// this [GraphQL enum][0].
    ///
    /// [`marker::IsOutputType`]: juniper::marker::IsOutputType
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    fn impl_input_and_output_type_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::marker::IsInputType<#scalar>
                for #ident #ty_generics
                #where_clause {}

            #[automatically_derived]
            impl #impl_generics ::juniper::marker::IsOutputType<#scalar>
                for #ident #ty_generics
                #where_clause {}
        }
    }

    /// Returns generated code implementing [`GraphQLType`] trait for this
    /// [GraphQL enum][0].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    fn impl_graphql_type_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let name = &self.name;
        let description = &self.description;

        let variants_meta = self.values.iter().map(|v| {
            let v_name = &v.name;
            let v_description = &v.description;
            let v_deprecation = &v.deprecated;

            quote! {
                ::juniper::meta::EnumValue::new(#v_name)
                    #v_description
                    #v_deprecation
            }
        });

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::GraphQLType<#scalar>
                for #ident #ty_generics
                #where_clause
            {
                fn name(
                    _ : &Self::TypeInfo,
                ) -> ::core::option::Option<&'static ::core::primitive::str> {
                    ::core::option::Option::Some(#name)
                }

                fn meta<'r>(
                    info: &Self::TypeInfo,
                    registry: &mut ::juniper::Registry<'r, #scalar>
                ) -> ::juniper::meta::MetaType<'r, #scalar>
                where #scalar: 'r,
                {
                    let variants = [#( #variants_meta ),*];

                    registry.build_enum_type::<#ident #ty_generics>(info, &variants)
                        #description
                        .into_meta()
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValue`] trait for this
    /// [GraphQL enum][0].
    ///
    /// [`GraphQLValue`]: juniper::GraphQLValue
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    fn impl_graphql_value_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;
        let context = &self.context;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let variants = self.values.iter().map(|v| {
            let ident = &v.ident;
            let name = &v.name;

            quote! {
                Self::#ident => ::core::result::Result::Ok(::juniper::Value::scalar(
                    ::std::string::String::from(#name),
                )),
            }
        });

        let ignored = self.has_ignored_variants.then(|| {
            quote! {
                _ => ::core::result::Result::Err(::juniper::FieldError::<#scalar>::from(
                    "Cannot resolve ignored enum variant",
                )),
            }
        });

        quote! {
            impl #impl_generics ::juniper::GraphQLValue<#scalar>
                for #ident #ty_generics
                #where_clause
            {
                type Context = #context;
                type TypeInfo = ();

                fn type_name<'__i>(
                    &self,
                    info: &'__i Self::TypeInfo,
                ) -> ::core::option::Option<&'__i ::core::primitive::str> {
                    <Self as ::juniper::GraphQLType<#scalar>>::name(info)
                }

                fn resolve(
                    &self,
                    _: &(),
                    _: ::core::option::Option<&[::juniper::Selection<#scalar>]>,
                    _: &::juniper::Executor<Self::Context, #scalar>,
                ) -> ::juniper::ExecutionResult<#scalar> {
                    match self {
                        #( #variants )*
                        #ignored
                    }
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValueAsync`] trait for this
    /// [GraphQL enum][0].
    ///
    /// [`GraphQLValueAsync`]: juniper::GraphQLValueAsync
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    fn impl_graphql_value_async_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(true);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        quote! {
            impl #impl_generics ::juniper::GraphQLValueAsync<#scalar>
                for #ident #ty_generics
                #where_clause
            {
                fn resolve_async<'__a>(
                    &'__a self,
                    info: &'__a Self::TypeInfo,
                    selection_set: ::core::option::Option<&'__a [::juniper::Selection<#scalar>]>,
                    executor: &'__a ::juniper::Executor<Self::Context, #scalar>,
                ) -> ::juniper::BoxFuture<'__a, ::juniper::ExecutionResult<#scalar>> {
                    let v = ::juniper::GraphQLValue::resolve(self, info, selection_set, executor);
                    ::std::boxed::Box::pin(::juniper::futures::future::ready(v))
                }
            }
        }
    }

    /// Returns generated code implementing [`FromInputValue`] trait for this
    /// [GraphQL enum][0].
    ///
    /// [`FromInputValue`]: juniper::FromInputValue
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    fn impl_from_input_value_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let variants = self.values.iter().map(|v| {
            let ident = &v.ident;
            let name = &v.name;

            quote! {
                ::core::option::Option::Some(#name) => ::core::result::Result::Ok(Self::#ident),
            }
        });

        quote! {
            impl #impl_generics ::juniper::FromInputValue<#scalar>
                for #ident #ty_generics
                #where_clause
            {
                type Error = ::std::string::String;

                fn from_input_value(
                    v: &::juniper::InputValue<#scalar>,
                ) -> ::core::result::Result<Self, Self::Error> {
                    match v.as_enum_value().or_else(|| v.as_string_value()) {
                        #( #variants )*
                        _ => ::core::result::Result::Err(
                            ::std::format!("Unknown enum value: {}", v),
                        ),
                    }
                }
            }
        }
    }

    /// Returns generated code implementing [`ToInputValue`] trait for this
    /// [GraphQL enum][0].
    ///
    /// [`ToInputValue`]: juniper::ToInputValue
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    fn impl_to_input_value_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let variants = self.values.iter().map(|v| {
            let var_ident = &v.ident;
            let name = &v.name;

            quote! {
                #ident::#var_ident => ::juniper::InputValue::<#scalar>::scalar(
                    ::std::string::String::from(#name),
                ),
            }
        });

        let ignored = self.has_ignored_variants.then(|| {
            quote! {
                _ => ::core::panic!("Cannot resolve ignored enum variant"),
            }
        });

        quote! {
            impl #impl_generics ::juniper::ToInputValue<#scalar>
                for #ident #ty_generics
                #where_clause
            {
                fn to_input_value(&self) -> ::juniper::InputValue<#scalar> {
                    match self {
                        #( #variants )*
                        #ignored
                    }
                }
            }
        }
    }

    /// Returns generated code implementing [`BaseType`], [`BaseSubTypes`] and
    /// [`WrappedType`] traits for this [GraphQL enum][0].
    ///
    /// [`BaseSubTypes`]: juniper::macros::reflect::BaseSubTypes
    /// [`BaseType`]: juniper::macros::reflect::BaseType
    /// [`WrappedType`]: juniper::macros::reflect::WrappedType
    /// [0]: https://spec.graphql.org/October2021#sec-Enums
    fn impl_reflection_traits_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let name = &self.name;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        quote! {
            impl #impl_generics ::juniper::macros::reflect::BaseType<#scalar>
                for #ident #ty_generics
                #where_clause
            {
                const NAME: ::juniper::macros::reflect::Type = #name;
            }

            impl #impl_generics ::juniper::macros::reflect::BaseSubTypes<#scalar>
                for #ident #ty_generics
                #where_clause
            {
                const NAMES: ::juniper::macros::reflect::Types =
                    &[<Self as ::juniper::macros::reflect::BaseType<#scalar>>::NAME];
            }

            impl #impl_generics ::juniper::macros::reflect::WrappedType<#scalar>
                for #ident #ty_generics
                #where_clause
            {
                const VALUE: ::juniper::macros::reflect::WrappedValue = 1;
            }
        }
    }

    /// Returns prepared [`syn::Generics`] for [`GraphQLType`] trait (and
    /// similar) implementation of this enum.
    ///
    /// If `for_async` is `true`, then additional predicates are added to suit
    /// the [`GraphQLAsyncValue`] trait (and similar) requirements.
    ///
    /// [`GraphQLAsyncValue`]: juniper::GraphQLAsyncValue
    /// [`GraphQLType`]: juniper::GraphQLType
    fn impl_generics(&self, for_async: bool) -> syn::Generics {
        let mut generics = self.generics.clone();

        let scalar = &self.scalar;
        if scalar.is_implicit_generic() {
            generics.params.push(parse_quote! { #scalar });
        }
        if scalar.is_generic() {
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote! { #scalar: ::juniper::ScalarValue });
        }
        if let Some(bound) = scalar.bounds() {
            generics.make_where_clause().predicates.push(bound);
        }

        if for_async {
            let self_ty = if self.generics.lifetimes().next().is_some() {
                // Modify lifetime names to omit "lifetime name `'a` shadows a
                // lifetime name that is already in scope" error.
                let mut generics = self.generics.clone();
                for lt in generics.lifetimes_mut() {
                    let ident = lt.lifetime.ident.unraw();
                    lt.lifetime.ident = format_ident!("__fa__{ident}");
                }

                let lifetimes = generics.lifetimes().map(|lt| &lt.lifetime);
                let ident = &self.ident;
                let (_, ty_generics, _) = generics.split_for_impl();

                quote! { for<#( #lifetimes ),*> #ident #ty_generics }
            } else {
                quote! { Self }
            };
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote! { #self_ty: ::core::marker::Sync });

            if scalar.is_generic() {
                generics
                    .make_where_clause()
                    .predicates
                    .push(parse_quote! { #scalar: ::core::marker::Send + ::core::marker::Sync });
            }
        }

        generics
    }
}
