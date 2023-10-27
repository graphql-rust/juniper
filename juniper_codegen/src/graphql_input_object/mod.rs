//! Code generation for [GraphQL input objects][0].
//!
//! [0]: https://spec.graphql.org/October2021#sec-Input-Objects

pub(crate) mod derive;

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    ext::IdentExt as _,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
    token,
};

use crate::common::{
    default, filter_attrs,
    parse::{
        attr::{err, OptionExt as _},
        ParseBufferExt as _,
    },
    rename, scalar, Description, SpanContainer,
};

/// Available arguments behind `#[graphql]` attribute placed on a Rust struct
/// definition, when generating code for a [GraphQL input object][0].
///
/// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
#[derive(Debug, Default)]
struct ContainerAttr {
    /// Explicitly specified name of this [GraphQL input object][0].
    ///
    /// If [`None`], then Rust struct name will be used by default.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    name: Option<SpanContainer<String>>,

    /// Explicitly specified [description][2] of this [GraphQL input object][0].
    ///
    /// If [`None`], then Rust doc comment will be used as the [description][2],
    /// if any.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    description: Option<SpanContainer<Description>>,

    /// Explicitly specified type of [`Context`] to use for resolving this
    /// [GraphQL input object][0] type with.
    ///
    /// If [`None`], then unit type `()` is assumed as a type of [`Context`].
    ///
    /// [`Context`]: juniper::Context
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    context: Option<SpanContainer<syn::Type>>,

    /// Explicitly specified type (or type parameter with its bounds) of
    /// [`ScalarValue`] to use for resolving this [GraphQL input object][0] type
    /// with.
    ///
    /// If [`None`], then generated code will be generic over any
    /// [`ScalarValue`] type.
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    scalar: Option<SpanContainer<scalar::AttrValue>>,

    /// Explicitly specified [`rename::Policy`] for all fields of this
    /// [GraphQL input object][0].
    ///
    /// If [`None`], then the [`rename::Policy::CamelCase`] will be applied by
    /// default.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    rename_fields: Option<SpanContainer<rename::Policy>>,

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
                    out.rename_fields
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
            rename_fields: try_merge_opt!(rename_fields: self, another),
            is_internal: self.is_internal || another.is_internal,
        })
    }

    /// Parses [`ContainerAttr`] from the given multiple `name`d
    /// [`syn::Attribute`]s placed on a struct or impl block definition.
    pub(crate) fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
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
/// [GraphQL input object][0]'s [field][1].
///
/// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
/// [1]: https://spec.graphql.org/October2021#InputFieldsDefinition
#[derive(Debug, Default)]
struct FieldAttr {
    /// Explicitly specified name of this [GraphQL input object field][1].
    ///
    /// If [`None`], then Rust struct field name will be used by default.
    ///
    /// [1]: https://spec.graphql.org/October2021#InputValueDefinition
    name: Option<SpanContainer<String>>,

    /// Explicitly specified [default value][2] of this
    /// [GraphQL input object field][1] to be used used in case a field value is
    /// not provided.
    ///
    /// If [`None`], the this [field][1] will have no [default value][2].
    ///
    /// [1]: https://spec.graphql.org/October2021#InputValueDefinition
    /// [2]: https://spec.graphql.org/October2021#DefaultValue
    default: Option<SpanContainer<default::Value>>,

    /// Explicitly specified [description][2] of this
    /// [GraphQL input object field][1].
    ///
    /// If [`None`], then Rust doc comment will be used as the [description][2],
    /// if any.
    ///
    /// [1]: https://spec.graphql.org/October2021#InputValueDefinition
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    description: Option<SpanContainer<Description>>,

    /// Explicitly specified marker for the Rust struct field to be ignored and
    /// not included into the code generated for a [GraphQL input object][0]
    /// implementation.
    ///
    /// Ignored Rust struct fields still consider the [`default`] attribute's
    /// argument.
    ///
    /// [`default`]: Self::default
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    ignore: Option<SpanContainer<syn::Ident>>,
}

impl Parse for FieldAttr {
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
                "default" => {
                    let val = input.parse::<default::Value>()?;
                    out.default
                        .replace(SpanContainer::new(ident.span(), Some(val.span()), val))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "desc" | "description" => {
                    input.parse::<token::Eq>()?;
                    let desc = input.parse::<Description>()?;
                    out.description
                        .replace(SpanContainer::new(ident.span(), Some(desc.span()), desc))
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

impl FieldAttr {
    /// Tries to merge two [`FieldAttr`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            default: try_merge_opt!(default: self, another),
            description: try_merge_opt!(description: self, another),
            ignore: try_merge_opt!(ignore: self, another),
        })
    }

    /// Parses [`FieldAttr`] from the given multiple `name`d [`syn::Attribute`]s
    /// placed on a trait definition.
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

/// Representation of a [GraphQL input object field][1] for code generation.
///
/// [1]: https://spec.graphql.org/October2021#InputFieldsDefinition
#[derive(Debug)]
struct FieldDefinition {
    /// [`Ident`] of the Rust struct field behind this
    /// [GraphQL input object field][1].
    ///
    /// [`Ident`]: syn::Ident
    /// [1]: https://spec.graphql.org/October2021#InputValueDefinition
    ident: syn::Ident,

    /// Rust type that this [GraphQL input object field][1] is represented with.
    ///
    /// It should contain all its generics, if any.
    ///
    /// [1]: https://spec.graphql.org/October2021#InputValueDefinition
    ty: syn::Type,

    /// [Default value][2] of this [GraphQL input object field][1] to be used in
    /// case a [field][1] value is not provided.
    ///
    /// [1]: https://spec.graphql.org/October2021#InputValueDefinition
    /// [2]: https://spec.graphql.org/October2021#DefaultValue
    default: Option<default::Value>,

    /// Name of this [GraphQL input object field][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#InputValueDefinition
    name: Box<str>,

    /// [Description][2] of this [GraphQL input object field][1] to put into
    /// GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#InputValueDefinition
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    description: Option<Description>,

    /// Indicator whether the Rust struct field behinds this
    /// [GraphQL input object field][1] is being ignored and should not be
    /// included into the generated code.
    ///
    /// Ignored Rust struct fields still consider the [`default`] attribute's
    /// argument.
    ///
    /// [`default`]: Self::default
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    ignored: bool,
}

/// Representation of [GraphQL input object][0] for code generation.
///
/// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
#[derive(Debug)]
struct Definition {
    /// [`Ident`] of the Rust struct behind this [GraphQL input object][0].
    ///
    /// [`Ident`]: syn::Ident
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    ident: syn::Ident,

    /// [`Generics`] of the Rust enum behind this [GraphQL input object][0].
    ///
    /// [`Generics`]: syn::Generics
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    generics: syn::Generics,

    /// Name of this [GraphQL input object][0] in GraphQL schema.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    name: Box<str>,

    /// [Description][2] of this [GraphQL input object][0] to put into GraphQL
    /// schema.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    /// [2]: https://spec.graphql.org/October2021#sec-Descriptions
    description: Option<Description>,

    /// Rust type of [`Context`] to generate [`GraphQLType`] implementation with
    /// for this [GraphQL input object][0].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`Context`]: juniper::Context
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    context: syn::Type,

    /// [`ScalarValue`] parametrization to generate [`GraphQLType`]
    /// implementation with for this [GraphQL input object][0].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    scalar: scalar::Type,

    /// [Fields][1] of this [GraphQL input object][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    /// [1]: https://spec.graphql.org/October2021#InputFieldsDefinition
    fields: Vec<FieldDefinition>,
}

impl ToTokens for Definition {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.impl_input_type_tokens().to_tokens(into);
        self.impl_graphql_type_tokens().to_tokens(into);
        self.impl_graphql_value_tokens().to_tokens(into);
        self.impl_graphql_value_async_tokens().to_tokens(into);
        self.impl_from_input_value_tokens().to_tokens(into);
        self.impl_to_input_value_tokens().to_tokens(into);
        self.impl_reflection_traits_tokens().to_tokens(into);
    }
}

impl Definition {
    /// Returns generated code implementing [`marker::IsInputType`] trait for
    /// this [GraphQL input object][0].
    ///
    /// [`marker::IsInputType`]: juniper::marker::IsInputType
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    #[must_use]
    fn impl_input_type_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let assert_fields_input_values = self.fields.iter().filter_map(|f| {
            let ty = &f.ty;

            (!f.ignored).then(|| {
                quote! {
                    <#ty as ::juniper::marker::IsInputType<#scalar>>::mark();
                }
            })
        });

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::marker::IsInputType<#scalar>
                for #ident #ty_generics
                #where_clause
            {
                fn mark() {
                    #( #assert_fields_input_values )*
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLType`] trait for this
    /// [GraphQL input object][0].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    #[must_use]
    fn impl_graphql_type_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;
        let name = &self.name;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let description = &self.description;

        let fields = self.fields.iter().filter_map(|f| {
            let ty = &f.ty;
            let name = &f.name;

            (!f.ignored).then(|| {
                let arg = if let Some(default) = &f.default {
                    quote! { .arg_with_default::<#ty>(#name, &#default, info) }
                } else {
                    quote! { .arg::<#ty>(#name, info) }
                };
                let description = &f.description;

                quote! { registry #arg #description }
            })
        });

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::GraphQLType<#scalar>
                for #ident #ty_generics
                #where_clause
            {
                fn name(
                    _: &Self::TypeInfo,
                ) -> ::core::option::Option<&'static ::core::primitive::str> {
                    ::core::option::Option::Some(#name)
                }

                fn meta<'r>(
                    info: &Self::TypeInfo,
                    registry: &mut ::juniper::Registry<'r, #scalar>,
                ) -> ::juniper::meta::MetaType<'r, #scalar>
                where
                    #scalar: 'r,
                {
                    let fields = [#( #fields ),*];
                    registry
                        .build_input_object_type::<#ident #ty_generics>(info, &fields)
                        #description
                        .into_meta()
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValue`] trait for this
    /// [GraphQL input object][0].
    ///
    /// [`GraphQLValue`]: juniper::GraphQLValue
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    #[must_use]
    fn impl_graphql_value_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;
        let context = &self.context;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        quote! {
            #[automatically_derived]
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
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValueAsync`] trait for this
    /// [GraphQL input object][0].
    ///
    /// [`GraphQLValueAsync`]: juniper::GraphQLValueAsync
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    #[must_use]
    fn impl_graphql_value_async_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(true);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        quote! {
            #[allow(non_snake_case)]
            #[automatically_derived]
            impl #impl_generics ::juniper::GraphQLValueAsync<#scalar>
                for #ident #ty_generics
                #where_clause {}
        }
    }

    /// Returns generated code implementing [`FromInputValue`] trait for this
    /// [GraphQL input object][0].
    ///
    /// [`FromInputValue`]: juniper::FromInputValue
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    #[must_use]
    fn impl_from_input_value_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let fields = self.fields.iter().map(|f| {
            let ident = &f.ident;

            let construct = if f.ignored {
                f.default.as_ref().map_or_else(
                    || {
                        let expr = default::Value::default();
                        quote! { #expr }
                    },
                    |expr| quote! { #expr },
                )
            } else {
                let name = &f.name;

                let fallback = f.default.as_ref().map_or_else(
                    || {
                        quote! {
                            ::juniper::FromInputValue::<#scalar>::from_implicit_null()
                                .map_err(::juniper::IntoFieldError::into_field_error)?
                        }
                    },
                    |expr| quote! { #expr },
                );

                quote! {
                    match obj.get(#name) {
                        ::core::option::Option::Some(v) => {
                            ::juniper::FromInputValue::<#scalar>::from_input_value(v)
                                .map_err(::juniper::IntoFieldError::into_field_error)?
                        }
                        ::core::option::Option::None => { #fallback }
                    }
                }
            };

            quote! { #ident: { #construct }, }
        });

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::FromInputValue<#scalar>
                for #ident #ty_generics
                #where_clause
            {
                type Error = ::juniper::FieldError<#scalar>;

                fn from_input_value(
                    value: &::juniper::InputValue<#scalar>,
                ) -> ::core::result::Result<Self, Self::Error> {
                    let obj = value
                        .to_object_value()
                        .ok_or_else(|| ::juniper::FieldError::<#scalar>::from(
                            ::std::format!("Expected input object, found: {}", value))
                        )?;

                    ::core::result::Result::Ok(#ident {
                        #( #fields )*
                    })
                }
            }
        }
    }

    /// Returns generated code implementing [`ToInputValue`] trait for this
    /// [GraphQL input object][0].
    ///
    /// [`ToInputValue`]: juniper::ToInputValue
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    #[must_use]
    fn impl_to_input_value_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let fields = self.fields.iter().filter_map(|f| {
            let ident = &f.ident;
            let name = &f.name;

            (!f.ignored).then(|| {
                quote! {
                    (#name, ::juniper::ToInputValue::to_input_value(&self.#ident))
                }
            })
        });

        quote! {
            #[automatically_derived]
            impl #impl_generics ::juniper::ToInputValue<#scalar>
                for #ident #ty_generics
                #where_clause
            {
                fn to_input_value(&self) -> ::juniper::InputValue<#scalar> {
                    ::juniper::InputValue::object(
                        #[allow(deprecated)]
                        ::std::array::IntoIter::new([#( #fields ),*])
                            .collect()
                    )
                }
            }
        }
    }

    /// Returns generated code implementing [`BaseType`], [`BaseSubTypes`] and
    /// [`WrappedType`] traits for this [GraphQL input object][0].
    ///
    /// [`BaseSubTypes`]: juniper::macros::reflect::BaseSubTypes
    /// [`BaseType`]: juniper::macros::reflect::BaseType
    /// [`WrappedType`]: juniper::macros::reflect::WrappedType
    /// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
    #[must_use]
    fn impl_reflection_traits_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let name = &self.name;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        quote! {
            #[automatically_derived]
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
    /// similar) implementation of this struct.
    ///
    /// If `for_async` is `true`, then additional predicates are added to suit
    /// the [`GraphQLAsyncValue`] trait (and similar) requirements.
    ///
    /// [`GraphQLAsyncValue`]: juniper::GraphQLAsyncValue
    /// [`GraphQLType`]: juniper::GraphQLType
    #[must_use]
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
