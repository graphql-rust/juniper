//! Code generation for [GraphQL scalar][1].
//!
//! [1]: https://spec.graphql.org/October2021#sec-Scalars

use std::convert::TryFrom;

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

use crate::{
    common::{
        behavior,
        parse::{
            attr::{err, OptionExt as _},
            ParseBufferExt as _,
        },
        scalar,
    },
    util::{filter_attrs, get_doc_comment, span_container::SpanContainer},
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
    description: Option<SpanContainer<String>>,

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

    /// Explicitly specified type of the custom [`Behavior`] to parametrize this
    /// [GraphQL scalar][0] type implementation with.
    ///
    /// If [`None`], then [`behavior::Standard`] will be used for the generated
    /// code.
    ///
    /// [`Behavior`]: juniper::behavior
    /// [`behavior::Standard`]: juniper::behavior::Standard
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    behavior: Option<SpanContainer<behavior::Type>>,

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

    /// Indicator for single-field structs allowing to delegate implementations
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
                    let desc = input.parse::<syn::LitStr>()?;
                    out.description
                        .replace(SpanContainer::new(
                            ident.span(),
                            Some(desc.span()),
                            desc.value(),
                        ))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "specified_by_url" => {
                    input.parse::<token::Eq>()?;
                    let lit = input.parse::<syn::LitStr>()?;
                    let url = lit.value().parse::<Url>().map_err(|err| {
                        syn::Error::new(lit.span(), format!("Invalid URL: {}", err))
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
                "behave" | "behavior" => {
                    input.parse::<token::Eq>()?;
                    let bh = input.parse::<behavior::Type>()?;
                    out.behavior
                        .replace(SpanContainer::new(ident.span(), Some(bh.span()), bh))
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
                    let parsed_types =
                        types.parse_terminated::<_, token::Comma>(syn::Type::parse)?;

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
                            .parse_terminated::<_, token::Comma>(syn::WherePredicate::parse)?;

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
            behavior: try_merge_opt!(behavior: self, another),
            to_output: try_merge_opt!(to_output: self, another),
            from_input: try_merge_opt!(from_input: self, another),
            parse_token: try_merge_opt!(parse_token: self, another),
            with: try_merge_opt!(with: self, another),
            where_clause: try_merge_opt!(where_clause: self, another),
            transparent: self.transparent || another.transparent,
        })
    }

    /// Parses [`Attr`] from the given multiple `name`d [`syn::Attribute`]s
    /// placed on a trait definition.
    fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut attr = filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))?;

        if attr.description.is_none() {
            attr.description = get_doc_comment(attrs);
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
    description: Option<String>,

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

    /// [`ScalarValue`] parametrization to generate code with for this
    /// [GraphQL scalar][0].
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    scalar_value: ScalarValue,

    /// [`Behavior`] parametrization to generate code with for this
    /// [GraphQL scalar][0].
    ///
    /// [`Behavior`]: juniper::behavior
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    behavior: behavior::Type,
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
        ////////////////////////////////////////////////////////////////////////
        self.impl_resolve_type().to_tokens(into);
        self.impl_resolve_type_name().to_tokens(into);
        //self.impl_resolve_value().to_tokens(into);
        //self.impl_resolve_value_async().to_tokens(into);
        //self.impl_resolve_to_input_value().to_tokens(into);
        self.impl_resolve_input_value().to_tokens(into);
        self.impl_resolve_scalar_token().to_tokens(into);
        //self.impl_graphql_output_type().to_tokens(into);
        //self.impl_graphql_output_type().to_tokens(into);
        //self.impl_graphql_scalar().to_tokens(into);
        self.impl_reflect().to_tokens(into);
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
            impl#impl_gens ::juniper::marker::IsInputType<#scalar> for #ty
                #where_clause { }

            #[automatically_derived]
            impl#impl_gens ::juniper::marker::IsOutputType<#scalar> for #ty
                #where_clause { }
        }
    }

    /// Returns generated code implementing [`graphql::InputType`] and
    /// [`graphql::OutputType`] traits for this [GraphQL scalar][0].
    ///
    /// [`graphql::InputType`]: juniper::graphql::InputType
    /// [`graphql::OutputType`]: juniper::graphql::OutputType
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    #[must_use]
    fn impl_graphql_input_and_output_type(&self) -> TokenStream {
        let (ty, generics) = self.ty_and_generics();
        let (sv, generics) = self.mix_scalar_value(generics);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl#impl_gens ::juniper::graphql::InputType<#sv> for #ty
                #where_clause
            {
                fn assert_input_type() {}
            }

            #[automatically_derived]
            impl#impl_gens ::juniper::graphql::OutputType<#sv> for #ty
                #where_clause
            {
                fn assert_output_type() {}
            }
        }
    }

    /// Returns generated code implementing [`graphql::Scalar`] trait for this
    /// [GraphQL scalar][0].
    ///
    /// [`graphql::Scalar`]: juniper::graphql::Scalar
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    #[must_use]
    fn impl_graphql_scalar(&self) -> TokenStream {
        let (ty, generics) = self.ty_and_generics();
        let (sv, generics) = self.mix_scalar_value(generics);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl#impl_gens ::juniper::graphql::Scalar<#sv> for #ty
                #where_clause
            {
                fn assert_scalar() {}
            }
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

        let description = self
            .description
            .as_ref()
            .map(|val| quote! { .description(#val) });
        let specified_by_url = self.specified_by_url.as_ref().map(|url| {
            let url_lit = url.as_str();
            quote! { .specified_by_url(#url_lit) }
        });

        let (ty, generics) = self.impl_self_and_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl#impl_gens ::juniper::GraphQLType<#scalar> for #ty
                #where_clause
            {
                fn name(_: &Self::TypeInfo) -> Option<&'static str> {
                    Some(#name)
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

    /// Returns generated code implementing [`resolve::TypeName`] trait for this
    /// [GraphQL scalar][0].
    ///
    /// [`resolve::TypeName`]: juniper::resolve::TypeName
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_resolve_type_name(&self) -> TokenStream {
        let bh = &self.behavior;
        let (ty, generics) = self.ty_and_generics();
        let (inf, generics) = self.mix_type_info(generics);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl#impl_gens ::juniper::resolve::TypeName<#inf, #bh> for #ty
                #where_clause
            {
                fn type_name(_: &#inf) -> &'static str {
                    <Self as ::juniper::reflect::BaseType<#bh>>::NAME
                }
            }
        }
    }

    /// Returns generated code implementing [`resolve::Type`] trait for this
    /// [GraphQL scalar][0].
    ///
    /// [`resolve::Type`]: juniper::resolve::Type
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_resolve_type(&self) -> TokenStream {
        let bh = &self.behavior;
        let (ty, generics) = self.ty_and_generics();
        let (inf, generics) = self.mix_type_info(generics);
        let (sv, mut generics) = self.mix_scalar_value(generics);
        let predicates = &mut generics.make_where_clause().predicates;
        predicates.push(parse_quote! { #sv: Clone });
        predicates.push(parse_quote! {
            ::juniper::behavior::Coerce<Self>:
                ::juniper::resolve::TypeName<#inf>
                + ::juniper::resolve::ScalarToken<#sv>
                + ::juniper::resolve::InputValueOwned<#sv>
        });
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        let description = self
            .description
            .as_ref()
            .map(|val| quote! { .description(#val) });

        let specified_by_url = self.specified_by_url.as_ref().map(|url| {
            let url_lit = url.as_str();
            quote! { .specified_by_url(#url_lit) }
        });

        quote! {
            #[automatically_derived]
            impl#impl_gens ::juniper::resolve::Type<#inf, #sv, #bh> for #ty
                #where_clause
            {
                fn meta<'__r, '__ti: '__r>(
                    registry: &mut ::juniper::Registry<'__r, #sv>,
                    type_info: &'__ti #inf,
                ) -> ::juniper::meta::MetaType<'__r, #sv>
                where
                    #sv: '__r,
                {
                    registry.register_scalar_with::<
                        ::juniper::behavior::Coerce<Self>, _, _,
                    >(type_info, |meta| {
                        meta#description
                            #specified_by_url
                    })
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
            impl#impl_gens ::juniper::GraphQLValue<#scalar> for #ty
                #where_clause
            {
                type Context = ();
                type TypeInfo = ();

                fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
                    <Self as ::juniper::GraphQLType<#scalar>>::name(info)
                }

                fn resolve(
                    &self,
                    info: &(),
                    selection: Option<&[::juniper::Selection<'_, #scalar>]>,
                    executor: &::juniper::Executor<'_, '_, Self::Context, #scalar>,
                ) -> ::juniper::ExecutionResult<#scalar> {
                    #resolve
                }
            }
        }
    }

    /// Returns generated code implementing [`resolve::Value`] trait for this
    /// [GraphQL scalar][0].
    ///
    /// [`resolve::Value`]: juniper::resolve::Value
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_resolve_value(&self) -> TokenStream {
        let (ty, generics) = self.ty_and_generics();
        let (inf, generics) = self.mix_type_info(generics);
        let (cx, generics) = self.mix_context(generics);
        let (sv, mut generics) = self.mix_scalar_value(generics);
        generics
            .make_where_clause()
            .predicates
            .push(self.methods.bound_resolve_value(&inf, &cx, sv));
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        let body = self.methods.expand_resolve_value(&inf, &cx, sv);

        quote! {
            #[automatically_derived]
            impl#impl_gens ::juniper::resolve::Value<#inf, #cx, #sv> for #ty
                #where_clause
            {
                fn resolve_value(
                    &self,
                    selection: Option<&[::juniper::Selection<'_, #sv>]>,
                    info: &#inf,
                    executor: &::juniper::Executor<'_, '_, #cx, #sv>,
                ) -> ::juniper::ExecutionResult<#sv> {
                    #body
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
            impl#impl_gens ::juniper::GraphQLValueAsync<#scalar> for #ty
                #where_clause
            {
                fn resolve_async<'b>(
                    &'b self,
                    info: &'b Self::TypeInfo,
                    selection_set: Option<&'b [::juniper::Selection<'_, #scalar>]>,
                    executor: &'b ::juniper::Executor<'_, '_, Self::Context, #scalar>,
                ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<#scalar>> {
                    use ::juniper::futures::future;
                    let v = ::juniper::GraphQLValue::resolve(self, info, selection_set, executor);
                    Box::pin(future::ready(v))
                }
            }
        }
    }

    /// Returns generated code implementing [`resolve::ValueAsync`] trait for
    /// this [GraphQL scalar][0].
    ///
    /// [`resolve::ValueAsync`]: juniper::resolve::ValueAsync
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_resolve_value_async(&self) -> TokenStream {
        let (ty, generics) = self.ty_and_generics();
        let (inf, generics) = self.mix_type_info(generics);
        let (cx, generics) = self.mix_context(generics);
        let (sv, mut generics) = self.mix_scalar_value(generics);
        let preds = &mut generics.make_where_clause().predicates;
        preds.push(parse_quote! {
            Self: ::juniper::resolve::Value<#inf, #cx, #sv>
        });
        preds.push(parse_quote! {
            #sv: Send
        });
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl#impl_gens ::juniper::resolve::ValueAsync<#inf, #cx, #sv> for #ty
                #where_clause
            {
                fn resolve_value_async<'__r>(
                    &'__r self,
                    selection: Option<&'__r [::juniper::Selection<'_, #sv>]>,
                    info: &'__r #inf,
                    executor: &'__r ::juniper::Executor<'_, '_, #cx, #sv>,
                ) -> ::juniper::BoxFuture<'__r, ::juniper::ExecutionResult<#sv>> {
                    let v = <Self as ::juniper::resolve::Value<#inf, #cx, #sv>>
                        ::resolve_value(self, selection, info, executor);
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

        let to_input_value = self.methods.expand_old_to_input_value(scalar);

        let (ty, generics) = self.impl_self_and_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl#impl_gens ::juniper::ToInputValue<#scalar> for #ty
                #where_clause
            {
                fn to_input_value(&self) -> ::juniper::InputValue<#scalar> {
                    #to_input_value
                }
            }
        }
    }

    /// Returns generated code implementing [`resolve::ToInputValue`] trait for
    /// this [GraphQL scalar][0].
    ///
    /// [`resolve::ToInputValue`]: juniper::resolve::ToInputValue
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_resolve_to_input_value(&self) -> TokenStream {
        let (ty, generics) = self.ty_and_generics();
        let (sv, mut generics) = self.mix_scalar_value(generics);
        generics
            .make_where_clause()
            .predicates
            .push(self.methods.bound_to_input_value(sv));
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        let body = self.methods.expand_to_input_value(sv);

        quote! {
            #[automatically_derived]
            impl#impl_gens ::juniper::resolve::ToInputValue<#sv> for #ty
                #where_clause
            {
                fn to_input_value(&self) -> ::juniper::graphql::InputValue<#sv> {
                    #body
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
            impl#impl_gens ::juniper::FromInputValue<#scalar> for #ty
                #where_clause
            {
                type Error = ::juniper::executor::FieldError<#scalar>;

                fn from_input_value(input: &::juniper::InputValue<#scalar>) -> Result<Self, Self::Error> {
                    #from_input_value
                        .map_err(::juniper::executor::IntoFieldError::<#scalar>::into_field_error)
                }
            }
        }
    }

    /// Returns generated code implementing [`resolve::InputValue`] trait for
    /// this [GraphQL scalar][0].
    ///
    /// [`resolve::InputValue`]: juniper::resolve::InputValue
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_resolve_input_value(&self) -> TokenStream {
        let bh = &self.behavior;
        let (ty, generics) = self.ty_and_generics();
        let (sv, mut generics) = self.mix_scalar_value(generics);
        let lt: syn::GenericParam = parse_quote! { '__inp };
        generics.params.push(lt.clone());
        let predicates = &mut generics.make_where_clause().predicates;
        predicates.push(parse_quote! { #sv: #lt });
        predicates.extend(self.methods.bound_try_from_input_value(&lt, sv, bh));
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        let error_ty = self.methods.expand_try_from_input_value_error(&lt, sv, bh);
        let body = self.methods.expand_try_from_input_value(sv, bh);

        quote! {
            #[automatically_derived]
            impl#impl_gens ::juniper::resolve::InputValue<#lt, #sv, #bh> for #ty
                #where_clause
            {
                type Error = #error_ty;

                fn try_from_input_value(
                    input: &#lt ::juniper::graphql::InputValue<#sv>,
                ) -> ::std::result::Result<Self, Self::Error> {
                    #body
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
            impl#impl_gens ::juniper::ParseScalarValue<#scalar> for #ty
                #where_clause
            {
                fn from_str(
                    token: ::juniper::parser::ScalarToken<'_>,
                ) -> ::juniper::ParseScalarResult<'_, #scalar> {
                    #from_str
                }
            }
        }
    }

    /// Returns generated code implementing [`resolve::ScalarToken`] trait for
    /// this [GraphQL scalar][0].
    ///
    /// [`resolve::ScalarToken`]: juniper::resolve::ScalarToken
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_resolve_scalar_token(&self) -> TokenStream {
        let bh = &self.behavior;
        let (ty, generics) = self.ty_and_generics();
        let (sv, mut generics) = self.mix_scalar_value(generics);
        generics
            .make_where_clause()
            .predicates
            .extend(self.methods.bound_parse_scalar_token(&sv, bh));
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        let body = self.methods.expand_parse_scalar_token(&sv, bh);

        quote! {
            #[automatically_derived]
            impl#impl_gens ::juniper::resolve::ScalarToken<#sv, #bh> for #ty
                #where_clause
            {
                fn parse_scalar_token(
                    token: ::juniper::parser::ScalarToken<'_>,
                ) -> ::std::result::Result<
                    #sv,
                    ::juniper::parser::ParseError<'_>,
                > {
                    #body
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
            impl#impl_gens ::juniper::macros::reflect::BaseType<#scalar> for #ty
                #where_clause
            {
                const NAME: ::juniper::macros::reflect::Type = #name;
            }

            #[automatically_derived]
            impl#impl_gens ::juniper::macros::reflect::BaseSubTypes<#scalar> for #ty
                #where_clause
            {
                const NAMES: ::juniper::macros::reflect::Types =
                    &[<Self as ::juniper::macros::reflect::BaseType<#scalar>>::NAME];
            }

            #[automatically_derived]
            impl#impl_gens ::juniper::macros::reflect::WrappedType<#scalar> for #ty
                #where_clause
            {
                const VALUE: ::juniper::macros::reflect::WrappedValue = 1;
            }
        }
    }

    /// Returns generated code implementing [`reflect::BaseType`],
    /// [`reflect::BaseSubTypes`] and [`reflect::WrappedType`] traits for this
    /// [GraphQL scalar][0].
    ///
    /// [`reflect::BaseSubTypes`]: juniper::reflection::BaseSubTypes
    /// [`reflect::BaseType`]: juniper::reflection::BaseType
    /// [`reflect::WrappedType`]: juniper::reflection::WrappedType
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    fn impl_reflect(&self) -> TokenStream {
        let bh = &self.behavior;
        let (ty, generics) = self.ty_and_generics();
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        let name = &self.name;

        quote! {
            #[automatically_derived]
            impl#impl_gens ::juniper::reflect::BaseType<#bh> for #ty
                #where_clause
            {
                const NAME: ::juniper::reflect::Type = #name;
            }

            #[automatically_derived]
            impl#impl_gens ::juniper::reflect::BaseSubTypes<#bh> for #ty
                #where_clause
            {
                const NAMES: ::juniper::reflect::Types =
                    &[<Self as ::juniper::reflect::BaseType<#bh>>::NAME];
            }

            #[automatically_derived]
            impl#impl_gens ::juniper::reflect::WrappedType<#bh> for #ty
                #where_clause
            {
                const VALUE: ::juniper::reflect::WrappedValue =
                    ::juniper::reflect::wrap::SINGULAR;
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
                quote! { #ident#ty_gen }
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
                        quote! { #ident#ty_gens }
                    }
                };

                quote! { for<#( #lifetimes ),*> #ty }
            } else {
                quote! { Self }
            };
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote! { #self_ty: Sync });

            if scalar.is_generic() {
                generics
                    .make_where_clause()
                    .predicates
                    .push(parse_quote! { #scalar: Send + Sync });
            }
        }

        (ty, generics)
    }

    /// Returns prepared self [`syn::Type`] and [`syn::Generics`] for a trait
    /// implementation.
    #[must_use]
    fn ty_and_generics(&self) -> (syn::Type, syn::Generics) {
        let mut generics = self.generics.clone();

        let ty = match &self.ty {
            TypeOrIdent::Type(ty) => (**ty).clone(),
            TypeOrIdent::Ident(ident) => {
                let (_, ty_gen, _) = self.generics.split_for_impl();
                parse_quote! { #ident#ty_gen }
            }
        };

        if !self.where_clause.is_empty() {
            generics
                .make_where_clause()
                .predicates
                .extend(self.where_clause.clone())
        }

        (ty, generics)
    }

    /// Mixes a type info [`syn::GenericParam`] into the provided
    /// [`syn::Generics`] and returns its [`syn::Ident`].
    #[must_use]
    fn mix_type_info(&self, mut generics: syn::Generics) -> (syn::Ident, syn::Generics) {
        let ty = parse_quote! { __TypeInfo };
        generics.params.push(parse_quote! { #ty: ?Sized });
        (ty, generics)
    }

    /// Mixes a context [`syn::GenericParam`] into the provided
    /// [`syn::Generics`] and returns its [`syn::Ident`].
    #[must_use]
    fn mix_context(&self, mut generics: syn::Generics) -> (syn::Ident, syn::Generics) {
        let ty = parse_quote! { __Context };
        generics.params.push(parse_quote! { #ty: ?Sized });
        (ty, generics)
    }

    /// Mixes a [`ScalarValue`] [`syn::GenericParam`] into the provided
    /// [`syn::Generics`] and returns it.
    ///
    /// [`ScalarValue`] trait bound is not made here, because some trait
    /// implementations may not require it, depending on the generated code or
    /// even at all.
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    #[must_use]
    fn mix_scalar_value(&self, mut generics: syn::Generics) -> (&ScalarValue, syn::Generics) {
        let sv = &self.scalar_value;
        generics.params.push(parse_quote! { #sv });
        (sv, generics)
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

/// User-provided methods for implementing a [GraphQL scalar][0].
///
/// [0]: https://spec.graphql.org/October2021#sec-Scalars
enum Methods {
    /// [GraphQL scalar][0] represented with custom resolving methods only.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    Custom {
        /// Function provided with `#[graphql(to_output_with = ...)]` attribute.
        to_output: syn::ExprPath,

        /// Function provided with `#[graphql(from_input_with = ...)]`
        /// attribute.
        from_input: syn::ExprPath,

        /// [`ParseToken`] provided with `#[graphql(parse_token_with = ...)]`
        /// or `#[graphql(parse_token(...))]` attribute.
        parse_token: ParseToken,
    },

    /// [GraphQL scalar][0] maybe partially represented with custom resolving
    /// methods. Other methods are re-used from its inner [`Field`].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
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
                quote! { Ok(#to_output(self)) }
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

    /// Expands body of [`resolve::Value::resolve_value()`][0] method.
    ///
    /// [0]: juniper::resolve::Value::resolve_value
    fn expand_resolve_value(
        &self,
        inf: &syn::Ident,
        cx: &syn::Ident,
        sv: &ScalarValue,
    ) -> TokenStream {
        match self {
            Self::Custom { to_output, .. }
            | Self::Delegated {
                to_output: Some(to_output),
                ..
            } => {
                quote! { Ok(#to_output(self)) }
            }

            Self::Delegated { field, .. } => {
                let field_ty = field.ty();

                quote! {
                    <#field_ty as ::juniper::resolve::Value<#inf, #cx, #sv>>
                        ::resolve_value(
                            &self.#field,
                            info,
                            selection,
                            executor,
                        )
                }
            }
        }
    }

    /// Generates additional trait bounds for [`resolve::Value`] implementation
    /// allowing to execute [`resolve::Value::resolve_value()`][0] method.
    ///
    /// [`resolve::Value`]: juniper::resolve::Value
    /// [0]: juniper::resolve::Value::resolve_value
    fn bound_resolve_value(
        &self,
        inf: &syn::Ident,
        cx: &syn::Ident,
        sv: &ScalarValue,
    ) -> syn::WherePredicate {
        match self {
            Self::Custom { .. }
            | Self::Delegated {
                to_output: Some(_), ..
            } => {
                parse_quote! {
                    #sv: ::juniper::ScalarValue
                }
            }

            Self::Delegated { field, .. } => {
                let field_ty = field.ty();

                parse_quote! {
                    #field_ty: ::juniper::resolve::Value<#inf, #cx, #sv>
                }
            }
        }
    }

    /// Expands [`ToInputValue::to_input_value`] method.
    ///
    /// [`ToInputValue::to_input_value`]: juniper::ToInputValue::to_input_value
    fn expand_old_to_input_value(&self, scalar: &scalar::Type) -> TokenStream {
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

    /// Expands body of [`resolve::ToInputValue::to_input_value()`][0] method.
    ///
    /// [0]: juniper::resolve::ToInputValue::to_input_value
    fn expand_to_input_value(&self, sv: &ScalarValue) -> TokenStream {
        match self {
            Self::Custom { to_output, .. }
            | Self::Delegated {
                to_output: Some(to_output),
                ..
            } => {
                quote! {
                    let v = #to_output(self);
                    ::juniper::resolve::ToInputValue::<#sv>::to_input_value(&v)
                }
            }

            Self::Delegated { field, .. } => {
                let field_ty = field.ty();

                quote! {
                    <#field_ty as ::juniper::resolve::ToInputValue<#sv>>
                        ::to_input_value(&self.#field)
                }
            }
        }
    }

    /// Generates additional trait bounds for [`resolve::ToInputValue`]
    /// implementation allowing to execute
    /// [`resolve::ToInputValue::to_input_value()`][0] method.
    ///
    /// [`resolve::ToInputValue`]: juniper::resolve::ToInputValue
    /// [0]: juniper::resolve::ToInputValue::to_input_value
    fn bound_to_input_value(&self, sv: &ScalarValue) -> syn::WherePredicate {
        match self {
            Self::Custom { .. }
            | Self::Delegated {
                to_output: Some(_), ..
            } => {
                parse_quote! {
                    #sv: ::juniper::ScalarValue
                }
            }

            Self::Delegated { field, .. } => {
                let field_ty = field.ty();

                parse_quote! {
                    #field_ty: ::juniper::resolve::ToInputValue<#sv>>
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

    /// Expands body of [`resolve::InputValue::try_from_input_value()`][0]
    /// method.
    ///
    /// [0]: juniper::resolve::InputValue::try_from_input_value
    fn expand_try_from_input_value(&self, sv: &ScalarValue, bh: &behavior::Type) -> TokenStream {
        match self {
            Self::Custom { from_input, .. }
            | Self::Delegated {
                from_input: Some(from_input),
                ..
            } => {
                let map_sv = sv.custom.is_some().then(|| {
                    quote! { .map_scalar_value() }
                });
                quote! {
                    #from_input(input#map_sv)
                        .map_err(
                            ::juniper::IntoFieldError::<#sv>::into_field_error,
                        )
                }
            }

            Self::Delegated { field, .. } => {
                let field_ty = field.ty();
                let field_bh = &field.behavior;
                let self_constructor = field.closure_constructor();

                quote! {
                    <::juniper::behavior::Coerce<#field_ty, #bh> as
                     ::juniper::resolve::InputValue<'_, #sv, #field_bh>>
                        ::try_from_input_value(input)
                            .map(::juniper::behavior::Coerce::into_inner)
                            .map(#self_constructor)
                }
            }
        }
    }

    /// Expands error type of [`resolve::InputValue`] trait.
    ///
    /// [`resolve::InputValue`]: juniper::resolve::InputValue
    fn expand_try_from_input_value_error(
        &self,
        lt: &syn::GenericParam,
        sv: &ScalarValue,
        bh: &behavior::Type,
    ) -> syn::Type {
        match self {
            Self::Custom { .. }
            | Self::Delegated {
                from_input: Some(_),
                ..
            } => {
                parse_quote! {
                    ::juniper::FieldError<#sv>
                }
            }

            Self::Delegated { field, .. } => {
                let field_ty = field.ty();
                let field_bh = &field.behavior;

                parse_quote! {
                    <::juniper::behavior::Coerce<#field_ty, #bh> as
                     ::juniper::resolve::InputValue<#lt, #sv, #field_bh>>::Error
                }
            }
        }
    }

    /// Generates additional trait bounds for [`resolve::InputValue`]
    /// implementation allowing to execute
    /// [`resolve::InputValue::try_from_input_value()`][0] method.
    ///
    /// [`resolve::InputValue`]: juniper::resolve::InputValue
    /// [0]: juniper::resolve::InputValue::try_from_input_value
    fn bound_try_from_input_value(
        &self,
        lt: &syn::GenericParam,
        sv: &ScalarValue,
        bh: &behavior::Type,
    ) -> Vec<syn::WherePredicate> {
        match self {
            Self::Custom { .. }
            | Self::Delegated {
                from_input: Some(_),
                ..
            } => {
                let mut bounds = vec![parse_quote! {
                    #sv: ::juniper::ScalarValue
                }];
                if let Some(custom_sv) = &sv.custom {
                    bounds.push(parse_quote! {
                        #custom_sv: ::juniper::ScalarValue
                    });
                }
                bounds
            }

            Self::Delegated { field, .. } => {
                let field_ty = field.ty();
                let field_bh = &field.behavior;

                vec![parse_quote! {
                    ::juniper::behavior::Coerce<#field_ty, #bh>:
                        ::juniper::resolve::InputValue<#lt, #sv, #field_bh>
                }]
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

    /// Expands body of [`resolve::ScalarToken::parse_scalar_token()`][0]
    /// method.
    ///
    /// [0]: juniper::resolve::ScalarToken::parse_scalar_token
    fn expand_parse_scalar_token(&self, sv: &ScalarValue, bh: &behavior::Type) -> TokenStream {
        match self {
            Self::Custom { parse_token, .. }
            | Self::Delegated {
                parse_token: Some(parse_token),
                ..
            } => parse_token.expand_parse_scalar_token(sv, bh),

            Self::Delegated { field, .. } => {
                let field_ty = field.ty();
                let field_bh = &field.behavior;

                quote! {
                    <::juniper::behavior::Coerce<#field_ty, #bh> as
                     ::juniper::resolve::ScalarToken<#sv, #field_bh>>
                        ::parse_scalar_token(token)
                }
            }
        }
    }

    /// Generates additional trait bounds for [`resolve::ScalarToken`]
    /// implementation allowing to execute
    /// [`resolve::ScalarToken::parse_scalar_token()`][0] method.
    ///
    /// [`resolve::ScalarToken`]: juniper::resolve::ScalarToken
    /// [0]: juniper::resolve::ScalarToken::parse_scalar_token
    fn bound_parse_scalar_token(
        &self,
        sv: &ScalarValue,
        bh: &behavior::Type,
    ) -> Vec<syn::WherePredicate> {
        match self {
            Self::Custom { parse_token, .. }
            | Self::Delegated {
                parse_token: Some(parse_token),
                ..
            } => parse_token.bound_parse_scalar_token(sv, bh),

            Self::Delegated { field, .. } => {
                let field_ty = field.ty();
                let field_bh = &field.behavior;

                vec![parse_quote! {
                    ::juniper::behavior::Coerce<#field_ty, #bh>:
                        ::juniper::resolve::ScalarToken<#sv, #field_bh>
                }]
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

    /// Expands body of [`resolve::ScalarToken::parse_scalar_token()`][0]
    /// method.
    ///
    /// [0]: juniper::resolve::ScalarToken::parse_scalar_token
    fn expand_parse_scalar_token(&self, sv: &ScalarValue, bh: &behavior::Type) -> TokenStream {
        match self {
            Self::Custom(parse_token) => {
                let into = sv.custom.is_some().then(|| {
                    quote! { .map(::juniper::ScalarValue::into_another) }
                });
                quote! {
                    #parse_token(token)#into
                }
            }

            Self::Delegated(delegated) => delegated
                .iter()
                .fold(None, |acc, ty| {
                    acc.map_or_else(
                        || {
                            Some(quote! {
                                <::juniper::behavior::Coerce<#ty, #bh> as
                                 ::juniper::resolve::ScalarToken<#sv>>
                                    ::parse_scalar_token(token)
                            })
                        },
                        |prev| {
                            Some(quote! {
                                #prev.or_else(|_| {
                                    <::juniper::behavior::Coerce<#ty, #bh> as
                                     ::juniper::resolve::ScalarToken<#sv>>
                                        ::parse_scalar_token(token)
                                })
                            })
                        },
                    )
                })
                .unwrap_or_default(),
        }
    }

    /// Generates additional trait bounds for [`resolve::ScalarToken`]
    /// implementation allowing to execute
    /// [`resolve::ScalarToken::parse_scalar_token()`][0] method.
    ///
    /// [`resolve::ScalarToken`]: juniper::resolve::ScalarToken
    /// [0]: juniper::resolve::ScalarToken::parse_scalar_token
    fn bound_parse_scalar_token(
        &self,
        sv: &ScalarValue,
        bh: &behavior::Type,
    ) -> Vec<syn::WherePredicate> {
        match self {
            Self::Custom(_) => {
                let mut bounds = vec![parse_quote! {
                    #sv: ::juniper::ScalarValue
                }];
                if let Some(custom_sv) = &sv.custom {
                    bounds.push(parse_quote! {
                        #custom_sv: ::juniper::ScalarValue
                    });
                }
                bounds
            }

            Self::Delegated(delegated) => delegated
                .iter()
                .map(|ty| {
                    parse_quote! {
                        ::juniper::behavior::Coerce<#ty, #bh>:
                            ::juniper::resolve::ScalarToken<#sv>
                    }
                })
                .collect(),
        }
    }
}

/// Available arguments behind `#[graphql]` attribute on a [`Field`] when
/// generating code for a [GraphQL scalar][0] implementation.
///
/// [0]: https://spec.graphql.org/October2021#sec-Scalars
#[derive(Debug, Default)]
struct FieldAttr {
    /// Explicitly specified type of the custom [`Behavior`] used for
    /// [GraphQL scalar][0] implementation by the [`Field`].
    ///
    /// If [`None`], then [`behavior::Standard`] will be used for the generated
    /// code.
    ///
    /// [`Behavior`]: juniper::behavior
    /// [`behavior::Standard`]: juniper::behavior::Standard
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    behavior: Option<SpanContainer<behavior::Type>>,
}

impl Parse for FieldAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut out = Self::default();
        while !input.is_empty() {
            let ident = input.parse_any_ident()?;
            match ident.to_string().as_str() {
                "behave" | "behavior" => {
                    input.parse::<token::Eq>()?;
                    let bh = input.parse::<behavior::Type>()?;
                    out.behavior
                        .replace(SpanContainer::new(ident.span(), Some(bh.span()), bh))
                        .none_or_else(|_| err::dup_arg(&ident))?
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

impl FieldAttr {
    /// Tries to merge two [`FieldAttr`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            behavior: try_merge_opt!(behavior: self, another),
        })
    }

    /// Parses [`FieldAttr`] from the given multiple `name`d [`syn::Attribute`]s
    /// placed on a field definition.
    fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))
    }
}

/// Inner field of a type implementing [GraphQL scalar][0], that the
/// implementation delegates calls to.
///
/// [0]: https://spec.graphql.org/October2021#sec-Scalars
struct Field {
    /// This [`Field`] itself.
    itself: syn::Field,

    /// [`Behavior`] parametrization of this [`Field`].
    ///
    /// [`Behavior`]: juniper::behavior
    behavior: behavior::Type,
}

impl TryFrom<syn::Field> for Field {
    type Error = syn::Error;

    fn try_from(field: syn::Field) -> syn::Result<Self> {
        let attr = FieldAttr::from_attrs("graphql", &field.attrs)?;
        Ok(Self {
            itself: field,
            behavior: attr.behavior.map(|bh| bh.into_inner()).unwrap_or_default(),
        })
    }
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(name) = &self.itself.ident {
            name.to_tokens(tokens)
        } else {
            tokens.append(Literal::u8_unsuffixed(0))
        }
    }
}

impl Field {
    /// [`syn::Type`] of this [`Field`].
    fn ty(&self) -> &syn::Type {
        &self.itself.ty
    }

    /// Generates closure to construct a [GraphQL scalar][0] struct from an
    /// inner [`Field`] value.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Scalars
    fn closure_constructor(&self) -> TokenStream {
        if let Some(name) = &self.itself.ident {
            quote! { |v| Self { #name: v } }
        } else {
            quote! { Self }
        }
    }
}

/// [`ScalarValue`] parametrization of a [GraphQL scalar][0] implementation.
///
/// [`ScalarValue`]: juniper::ScalarValue
/// [0]: https://spec.graphql.org/October2021#sec-Scalars
struct ScalarValue {
    /// Concrete custom Rust type used in user-provided [`Methods`] as
    /// [`ScalarValue`].
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    custom: Option<syn::Type>,
}

impl ToTokens for ScalarValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        (quote! { __ScalarValue }).to_tokens(tokens)
    }
}

impl<'a> From<Option<&'a scalar::AttrValue>> for ScalarValue {
    fn from(attr: Option<&'a scalar::AttrValue>) -> Self {
        Self {
            custom: match attr {
                Some(scalar::AttrValue::Concrete(ty)) => Some(ty.clone()),
                Some(scalar::AttrValue::Generic(_)) | None => None,
            },
        }
    }
}
