//! Code generation for [GraphQL scalar][1].
//!
//! [1]: https://spec.graphql.org/October2021#sec-Scalars

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{
    ext::IdentExt as _,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned as _,
    token,
    visit_mut::VisitMut,
};
use url::Url;

use crate::common::{
    filter_attrs,
    parse::{
        attr::{err, OptionExt as _},
        ParseBufferExt as _,
    },
    scalar, AttrNames, Description, SpanContainer,
};

pub mod attr;
pub mod derive;

/// Available arguments behind `#[graphql]`/`#[graphql_scalar]` attributes when
/// generating code for [GraphQL scalar][1].
///
/// [1]: https://spec.graphql.org/October2021#sec-Scalars
#[derive(Debug, Default)]
struct Attr {
    /// Name of this [GraphQL scalar][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    name: Option<SpanContainer<String>>,

    /// Description of this [GraphQL scalar][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    description: Option<SpanContainer<Description>>,

    /// Spec [`Url`] of this [GraphQL scalar][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    specified_by_url: Option<SpanContainer<Url>>,

    /// Explicitly specified type (or type parameter with its bounds) of
    /// [`ScalarValue`] to use for resolving this [GraphQL scalar][1] type with.
    ///
    /// If [`None`], then generated code will be generic over any
    /// [`ScalarValue`] type, which, in turn, requires all [scalar][1] fields to
    /// be generic over any [`ScalarValue`] type too. That's why this type
    /// should be specified only if one of the variants implements
    /// [`GraphQLType`] in a non-generic way over [`ScalarValue`] type.
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    scalar: Option<SpanContainer<scalar::AttrValue>>,

    /// Explicitly specified function to be used as
    /// [`ToInputValue::to_input_value`] implementation.
    ///
    /// [`ToInputValue::to_input_value`]: juniper::ToInputValue::to_input_value
    to_output: Option<SpanContainer<syn::ExprPath>>,

    /// Explicitly specified function to be used as
    /// [`FromInputValue::from_input_value`] implementation.
    ///
    /// [`FromInputValue::from_input_value`]: juniper::FromInputValue::from_input_value
    from_input: Option<SpanContainer<syn::ExprPath>>,

    /// Explicitly specified resolver to be used as
    /// [`ParseScalarValue::from_str`] implementation.
    ///
    /// [`ParseScalarValue::from_str`]: juniper::ParseScalarValue::from_str
    parse_token: Option<SpanContainer<ParseToken>>,

    /// Explicitly specified module with all custom resolvers for
    /// [`Self::to_output`], [`Self::from_input`] and [`Self::parse_token`].
    with: Option<SpanContainer<syn::ExprPath>>,

    /// Explicit where clause added to [`syn::WhereClause`].
    where_clause: Option<SpanContainer<Vec<syn::WherePredicate>>>,

    /// Indicator for single-field structs allowing to delegate implmemntations
    /// of non-provided resolvers to that field.
    transparent: bool,
}

impl Parse for Attr {
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
                "specified_by_url" => {
                    input.parse::<token::Eq>()?;
                    let lit = input.parse::<syn::LitStr>()?;
                    let url = lit.value().parse::<Url>().map_err(|err| {
                        syn::Error::new(lit.span(), format!("Invalid URL: {err}"))
                    })?;
                    out.specified_by_url
                        .replace(SpanContainer::new(ident.span(), Some(lit.span()), url))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "scalar" | "Scalar" | "ScalarValue" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<scalar::AttrValue>()?;
                    out.scalar
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "to_output_with" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<syn::ExprPath>()?;
                    out.to_output
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "from_input_with" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<syn::ExprPath>()?;
                    out.from_input
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "parse_token_with" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<syn::ExprPath>()?;
                    out.parse_token
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(scl.span()),
                            ParseToken::Custom(scl),
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "parse_token" => {
                    let types;
                    let _ = syn::parenthesized!(types in input);
                    let parsed_types = types.parse_terminated(syn::Type::parse, token::Comma)?;

                    if parsed_types.is_empty() {
                        return Err(syn::Error::new(ident.span(), "expected at least 1 type."));
                    }

                    out.parse_token
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(parsed_types.span()),
                            ParseToken::Delegated(parsed_types.into_iter().collect()),
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "with" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<syn::ExprPath>()?;
                    out.with
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "where" => {
                    let (span, parsed_predicates) = {
                        let predicates;
                        let _ = syn::parenthesized!(predicates in input);
                        let parsed_predicates = predicates
                            .parse_terminated(syn::WherePredicate::parse, token::Comma)?;

                        if parsed_predicates.is_empty() {
                            return Err(syn::Error::new(
                                ident.span(),
                                "expected at least 1 where predicate",
                            ));
                        }

                        (
                            parsed_predicates.span(),
                            parsed_predicates.into_iter().collect(),
                        )
                    };

                    out.where_clause
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(span),
                            parsed_predicates,
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "transparent" => {
                    out.transparent = true;
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
            specified_by_url: try_merge_opt!(specified_by_url: self, another),
            scalar: try_merge_opt!(scalar: self, another),
            to_output: try_merge_opt!(to_output: self, another),
            from_input: try_merge_opt!(from_input: self, another),
            parse_token: try_merge_opt!(parse_token: self, another),
            with: try_merge_opt!(with: self, another),
            where_clause: try_merge_opt!(where_clause: self, another),
            transparent: self.transparent || another.transparent,
        })
    }

    /// Parses an [`Attr`] from the provided multiple [`syn::Attribute`]s with
    /// the specified `names`, placed on a type definition.
    fn from_attrs(names: impl AttrNames, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut attr = filter_attrs(names, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if attr.description.is_none() {
            attr.description = Description::parse_from_doc_attrs(attrs)?;
        }

        Ok(attr)
    }
}

/// [`syn::Type`] in case of `#[graphql_scalar]` or [`syn::Ident`] in case of
/// `#[derive(GraphQLScalar)]`.
#[derive(Clone)]
enum TypeOrIdent {
    /// [`syn::Type`].
    Type(Box<syn::Type>),

    /// [`syn::Ident`].
    Ident(syn::Ident),
}

/// Definition of [GraphQL scalar][1] for code generation.
///
/// [1]: https://spec.graphql.org/October2021#sec-Scalars
struct Definition {
    /// Name of this [GraphQL scalar][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    name: String,

    /// [`TypeOrIdent`] of this [GraphQL scalar][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    ty: TypeOrIdent,

    /// Additional [`Self::generics`] [`syn::WhereClause`] predicates.
    where_clause: Vec<syn::WherePredicate>,

    /// Generics of the Rust type that this [GraphQL scalar][1] is implemented
    /// for.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    generics: syn::Generics,

    /// [`GraphQLScalarMethods`] representing [GraphQL scalar][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    methods: Methods,

    /// Description of this [GraphQL scalar][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    description: Option<Description>,

    /// Spec [`Url`] of this [GraphQL scalar][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    specified_by_url: Option<Url>,

    /// [`ScalarValue`] parametrization to generate [`GraphQLType`]
    /// implementation with for this [GraphQL scalar][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    scalar: scalar::Type,
}

impl ToTokens for Definition {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.impl_output_and_input_type_tokens().to_tokens(into);
        self.impl_type_tokens().to_tokens(into);
        self.impl_value_tokens().to_tokens(into);
        self.impl_value_async_tokens().to_tokens(into);
        self.impl_to_input_value_tokens().to_tokens(into);
        self.impl_from_input_value_tokens().to_tokens(into);
        self.impl_parse_scalar_value_tokens().to_tokens(into);
        self.impl_reflection_traits_tokens().to_tokens(into);
    }
}

impl Definition {
    /// Returns generated code implementing [`marker::IsInputType`] and
    /// [`marker::IsOutputType`] trait for this [GraphQL scalar][1].
    ///
    /// [`marker::IsInputType`]: juniper::marker::IsInputType
    /// [`marker::IsOutputType`]: juniper::marker::IsOutputType
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    #[must_use]
    fn impl_output_and_input_type_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let (ty, generics) = self.impl_self_and_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_gens ::juniper::marker::IsInputType<#scalar> for #ty
                #where_clause { }

            #[automatically_derived]
            impl #impl_gens ::juniper::marker::IsOutputType<#scalar> for #ty
                #where_clause { }
        }
    }

    /// Returns generated code implementing [`GraphQLType`] trait for this
    /// [GraphQL scalar][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_type_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let name = &self.name;
        let description = &self.description;
        let specified_by_url = self.specified_by_url.as_ref().map(|url| {
            let url_lit = url.as_str();
            quote! { .specified_by_url(#url_lit) }
        });

        let (ty, generics) = self.impl_self_and_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_gens ::juniper::GraphQLType<#scalar> for #ty
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
                    registry.build_scalar_type::<Self>(info)
                        #description
                        #specified_by_url
                        .into_meta()
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValue`] trait for this
    /// [GraphQL scalar][1].
    ///
    /// [`GraphQLValue`]: juniper::GraphQLValue
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_value_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let resolve = self.methods.expand_resolve(scalar);

        let (ty, generics) = self.impl_self_and_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_gens ::juniper::GraphQLValue<#scalar> for #ty
                #where_clause
            {
                type Context = ();
                type TypeInfo = ();

                fn type_name<'i>(
                    &self,
                    info: &'i Self::TypeInfo,
                ) -> ::core::option::Option<&'i ::core::primitive::str> {
                    <Self as ::juniper::GraphQLType<#scalar>>::name(info)
                }

                fn resolve(
                    &self,
                    info: &(),
                    selection: ::core::option::Option<&[::juniper::Selection<'_, #scalar>]>,
                    executor: &::juniper::Executor<'_, '_, Self::Context, #scalar>,
                ) -> ::juniper::ExecutionResult<#scalar> {
                    #resolve
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValueAsync`] trait for this
    /// [GraphQL scalar][1].
    ///
    /// [`GraphQLValueAsync`]: juniper::GraphQLValueAsync
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_value_async_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let (ty, generics) = self.impl_self_and_generics(true);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_gens ::juniper::GraphQLValueAsync<#scalar> for #ty
                #where_clause
            {
                fn resolve_async<'b>(
                    &'b self,
                    info: &'b Self::TypeInfo,
                    selection_set: ::core::option::Option<&'b [::juniper::Selection<'_, #scalar>]>,
                    executor: &'b ::juniper::Executor<'_, '_, Self::Context, #scalar>,
                ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<#scalar>> {
                    let v = ::juniper::GraphQLValue::resolve(self, info, selection_set, executor);
                    ::std::boxed::Box::pin(::juniper::futures::future::ready(v))
                }
            }
        }
    }

    /// Returns generated code implementing [`InputValue`] trait for this
    /// [GraphQL scalar][1].
    ///
    /// [`InputValue`]: juniper::InputValue
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_to_input_value_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let to_input_value = self.methods.expand_to_input_value(scalar);

        let (ty, generics) = self.impl_self_and_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_gens ::juniper::ToInputValue<#scalar> for #ty
                #where_clause
            {
                fn to_input_value(&self) -> ::juniper::InputValue<#scalar> {
                    #to_input_value
                }
            }
        }
    }

    /// Returns generated code implementing [`FromInputValue`] trait for this
    /// [GraphQL scalar][1].
    ///
    /// [`FromInputValue`]: juniper::FromInputValue
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_from_input_value_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let from_input_value = self.methods.expand_from_input_value(scalar);

        let (ty, generics) = self.impl_self_and_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_gens ::juniper::FromInputValue<#scalar> for #ty
                #where_clause
            {
                type Error = ::juniper::executor::FieldError<#scalar>;

                fn from_input_value(
                    input: &::juniper::InputValue<#scalar>,
                ) -> ::core::result::Result<Self, Self::Error> {
                    #from_input_value
                        .map_err(::juniper::executor::IntoFieldError::<#scalar>::into_field_error)
                }
            }
        }
    }

    /// Returns generated code implementing [`ParseScalarValue`] trait for this
    /// [GraphQL scalar][1].
    ///
    /// [`ParseScalarValue`]: juniper::ParseScalarValue
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_parse_scalar_value_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;

        let from_str = self.methods.expand_parse_scalar_value(scalar);

        let (ty, generics) = self.impl_self_and_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_gens ::juniper::ParseScalarValue<#scalar> for #ty
                #where_clause
            {
                fn from_str(
                    token: ::juniper::parser::ScalarToken<'_>,
                ) -> ::juniper::ParseScalarResult<#scalar> {
                    #from_str
                }
            }
        }
    }

    /// Returns generated code implementing [`BaseType`], [`BaseSubTypes`] and
    /// [`WrappedType`] traits for this [GraphQL scalar][1].
    ///
    /// [`BaseSubTypes`]: juniper::macros::reflection::BaseSubTypes
    /// [`BaseType`]: juniper::macros::reflection::BaseType
    /// [`WrappedType`]: juniper::macros::reflection::WrappedType
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_reflection_traits_tokens(&self) -> TokenStream {
        let scalar = &self.scalar;
        let name = &self.name;

        let (ty, generics) = self.impl_self_and_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_gens ::juniper::macros::reflect::BaseType<#scalar> for #ty
                #where_clause
            {
                const NAME: ::juniper::macros::reflect::Type = #name;
            }

            #[automatically_derived]
            impl #impl_gens ::juniper::macros::reflect::BaseSubTypes<#scalar> for #ty
                #where_clause
            {
                const NAMES: ::juniper::macros::reflect::Types =
                    &[<Self as ::juniper::macros::reflect::BaseType<#scalar>>::NAME];
            }

            #[automatically_derived]
            impl #impl_gens ::juniper::macros::reflect::WrappedType<#scalar> for #ty
                #where_clause
            {
                const VALUE: ::juniper::macros::reflect::WrappedValue = 1;
            }
        }
    }

    /// Returns prepared self type and [`syn::Generics`] for [`GraphQLType`]
    /// trait (and similar) implementation.
    ///
    /// If `for_async` is `true`, then additional predicates are added to suit
    /// the [`GraphQLAsyncValue`] trait (and similar) requirements.
    ///
    /// [`GraphQLAsyncValue`]: juniper::GraphQLAsyncValue
    /// [`GraphQLType`]: juniper::GraphQLType
    #[must_use]
    fn impl_self_and_generics(&self, for_async: bool) -> (TokenStream, syn::Generics) {
        let mut generics = self.generics.clone();

        let ty = match &self.ty {
            TypeOrIdent::Type(ty) => ty.into_token_stream(),
            TypeOrIdent::Ident(ident) => {
                let (_, ty_gen, _) = self.generics.split_for_impl();
                quote! { #ident #ty_gen }
            }
        };

        if !self.where_clause.is_empty() {
            generics
                .make_where_clause()
                .predicates
                .extend(self.where_clause.clone())
        }

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
                let mut generics = self.generics.clone();
                ModifyLifetimes.visit_generics_mut(&mut generics);

                let lifetimes = generics.lifetimes().map(|lt| &lt.lifetime);
                let ty = match self.ty.clone() {
                    TypeOrIdent::Type(mut ty) => {
                        ModifyLifetimes.visit_type_mut(&mut ty);
                        ty.into_token_stream()
                    }
                    TypeOrIdent::Ident(ident) => {
                        let (_, ty_gens, _) = generics.split_for_impl();
                        quote! { #ident #ty_gens }
                    }
                };

                quote! { for<#( #lifetimes ),*> #ty }
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

        (ty, generics)
    }
}

/// Adds `__fa__` prefix to all lifetimes to avoid "lifetime name `'a` shadows a
/// lifetime name that is already in scope" error.
struct ModifyLifetimes;

impl VisitMut for ModifyLifetimes {
    fn visit_lifetime_mut(&mut self, lf: &mut syn::Lifetime) {
        lf.ident = format_ident!("__fa__{}", lf.ident.unraw());
    }
}

/// Methods representing [GraphQL scalar][1].
///
/// [1]: https://spec.graphql.org/October2021#sec-Scalars
enum Methods {
    /// [GraphQL scalar][1] represented with only custom resolvers.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    Custom {
        /// Function provided with `#[graphql(to_output_with = ...)]`.
        to_output: syn::ExprPath,

        /// Function provided with `#[graphql(from_input_with = ...)]`.
        from_input: syn::ExprPath,

        /// [`ParseToken`] provided with `#[graphql(parse_token_with = ...)]`
        /// or `#[graphql(parse_token(...))]`.
        parse_token: ParseToken,
    },

    /// [GraphQL scalar][1] maybe partially represented with custom resolver.
    /// Other methods are used from [`Field`].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    Delegated {
        /// Function provided with `#[graphql(to_output_with = ...)]`.
        to_output: Option<syn::ExprPath>,

        /// Function provided with `#[graphql(from_input_with = ...)]`.
        from_input: Option<syn::ExprPath>,

        /// [`ParseToken`] provided with `#[graphql(parse_token_with = ...)]`
        /// or `#[graphql(parse_token(...))]`.
        parse_token: Option<ParseToken>,

        /// [`Field`] to resolve not provided methods.
        field: Box<Field>,
    },
}

impl Methods {
    /// Expands [`GraphQLValue::resolve`] method.
    ///
    /// [`GraphQLValue::resolve`]: juniper::GraphQLValue::resolve
    fn expand_resolve(&self, scalar: &scalar::Type) -> TokenStream {
        match self {
            Self::Custom { to_output, .. }
            | Self::Delegated {
                to_output: Some(to_output),
                ..
            } => {
                quote! { ::core::result::Result::Ok(#to_output(self)) }
            }
            Self::Delegated { field, .. } => {
                quote! {
                    ::juniper::GraphQLValue::<#scalar>::resolve(
                        &self.#field,
                        info,
                        selection,
                        executor,
                    )
                }
            }
        }
    }

    /// Expands [`ToInputValue::to_input_value`] method.
    ///
    /// [`ToInputValue::to_input_value`]: juniper::ToInputValue::to_input_value
    fn expand_to_input_value(&self, scalar: &scalar::Type) -> TokenStream {
        match self {
            Self::Custom { to_output, .. }
            | Self::Delegated {
                to_output: Some(to_output),
                ..
            } => {
                quote! {
                    let v = #to_output(self);
                    ::juniper::ToInputValue::to_input_value(&v)
                }
            }
            Self::Delegated { field, .. } => {
                quote! {
                    ::juniper::ToInputValue::<#scalar>::to_input_value(&self.#field)
                }
            }
        }
    }

    /// Expands [`FromInputValue::from_input_value`][1] method.
    ///
    /// [1]: juniper::FromInputValue::from_input_value
    fn expand_from_input_value(&self, scalar: &scalar::Type) -> TokenStream {
        match self {
            Self::Custom { from_input, .. }
            | Self::Delegated {
                from_input: Some(from_input),
                ..
            } => {
                quote! { #from_input(input) }
            }
            Self::Delegated { field, .. } => {
                let field_ty = field.ty();
                let self_constructor = field.closure_constructor();
                quote! {
                    <#field_ty as ::juniper::FromInputValue<#scalar>>::from_input_value(input)
                        .map(#self_constructor)
                }
            }
        }
    }

    /// Expands [`ParseScalarValue::from_str`] method.
    ///
    /// [`ParseScalarValue::from_str`]: juniper::ParseScalarValue::from_str
    fn expand_parse_scalar_value(&self, scalar: &scalar::Type) -> TokenStream {
        match self {
            Self::Custom { parse_token, .. }
            | Self::Delegated {
                parse_token: Some(parse_token),
                ..
            } => {
                let parse_token = parse_token.expand_from_str(scalar);
                quote! { #parse_token }
            }
            Self::Delegated { field, .. } => {
                let field_ty = field.ty();
                quote! {
                    <#field_ty as ::juniper::ParseScalarValue<#scalar>>::from_str(token)
                }
            }
        }
    }
}

/// Representation of [`ParseScalarValue::from_str`] method.
///
/// [`ParseScalarValue::from_str`]: juniper::ParseScalarValue::from_str
#[derive(Clone, Debug)]
enum ParseToken {
    /// Custom method.
    Custom(syn::ExprPath),

    /// Tries to parse using [`syn::Type`]s [`ParseScalarValue`] impls until
    /// first success.
    ///
    /// [`ParseScalarValue`]: juniper::ParseScalarValue
    Delegated(Vec<syn::Type>),
}

impl ParseToken {
    /// Expands [`ParseScalarValue::from_str`] method.
    ///
    /// [`ParseScalarValue::from_str`]: juniper::ParseScalarValue::from_str
    fn expand_from_str(&self, scalar: &scalar::Type) -> TokenStream {
        match self {
            Self::Custom(parse_token) => {
                quote! { #parse_token(token) }
            }
            Self::Delegated(delegated) => delegated
                .iter()
                .fold(None, |acc, ty| {
                    acc.map_or_else(
                        || Some(quote! { <#ty as ::juniper::ParseScalarValue<#scalar>>::from_str(token) }),
                        |prev| {
                            Some(quote! {
                                #prev.or_else(|_| {
                                    <#ty as ::juniper::ParseScalarValue<#scalar>>::from_str(token)
                                })
                            })
                        }
                    )
                })
                .unwrap_or_default(),
        }
    }
}

/// Struct field to resolve not provided methods.
enum Field {
    /// Named [`Field`].
    Named(syn::Field),

    /// Unnamed [`Field`].
    Unnamed(syn::Field),
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Named(f) => f.ident.to_tokens(tokens),
            Self::Unnamed(_) => tokens.append(Literal::u8_unsuffixed(0)),
        }
    }
}

impl Field {
    /// [`syn::Type`] of this [`Field`].
    fn ty(&self) -> &syn::Type {
        match self {
            Self::Named(f) | Self::Unnamed(f) => &f.ty,
        }
    }

    /// Closure to construct [GraphQL scalar][1] struct from [`Field`].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Scalars
    fn closure_constructor(&self) -> TokenStream {
        match self {
            Field::Named(syn::Field { ident, .. }) => {
                quote! { |v| Self { #ident: v } }
            }
            Field::Unnamed(_) => quote! { Self },
        }
    }
}
