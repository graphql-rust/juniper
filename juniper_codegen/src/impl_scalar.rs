//! Code generation for [GraphQL scalar][1].
//!
//! [1]: https://spec.graphql.org/October2021/#sec-Scalars

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
    token,
};
use url::Url;

use crate::{
    common::{
        parse::{
            self,
            attr::{err, OptionExt as _},
            ParseBufferExt as _,
        },
        scalar,
    },
    result::GraphQLScope,
    util::{filter_attrs, get_doc_comment, span_container::SpanContainer},
};

/// [`GraphQLScope`] of errors for `#[graphql_scalar]` macro.
const ERR: GraphQLScope = GraphQLScope::ImplScalar;

/// Expands `#[graphql_scalar]` macro into generated code.
pub(crate) fn expand(attr_args: TokenStream, body: TokenStream) -> syn::Result<TokenStream> {
    if let Ok(mut ast) = syn::parse2::<syn::ItemImpl>(body) {
        let attrs = parse::attr::unite(("graphql_scalar", &attr_args), &ast.attrs);
        ast.attrs = parse::attr::strip("graphql_scalar", ast.attrs);
        return expand_on_impl(attrs, ast);
    }

    Err(syn::Error::new(
        Span::call_site(),
        "#[graphql_scalar] attribute is applicable to impl trait only",
    ))
}

/// Expands `#[graphql_scalar]` macro placed on an implementation block.
fn expand_on_impl(attrs: Vec<syn::Attribute>, ast: syn::ItemImpl) -> syn::Result<TokenStream> {
    let attr = Attr::from_attrs("graphql_scalar", &attrs)?;

    let mut self_ty = ast.self_ty.clone();
    if let syn::Type::Group(group) = self_ty.as_ref() {
        self_ty = group.elem.clone();
    }

    let name = attr
        .name
        .map(SpanContainer::into_inner)
        .or_else(|| {
            if let syn::Type::Path(path) = self_ty.as_ref() {
                path.path
                    .segments
                    .last()
                    .map(|last| last.ident.unraw().to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            ERR.custom_error(
                self_ty.span(),
                "unable to find target for implementation target for `GraphQLScalar`",
            )
        })?;

    let (_, trait_ty, _) = ast.trait_.as_ref().ok_or_else(|| {
        ERR.custom_error(
            ast.impl_token.span(),
            "expected `GraphQLScalar` trait implementation",
        )
    })?;

    let scalar = get_scalar(trait_ty, &ast.generics);

    let mut out = ast.to_token_stream();
    Definition {
        impl_for_type: *ast.self_ty.clone(),
        generics: ast.generics.clone(),
        name,
        description: attr.description.as_deref().cloned(),
        scalar,
        specified_by_url: attr.specified_by_url.as_deref().cloned(),
    }
    .to_tokens(&mut out);

    Ok(out)
}

/// Extracts [`scalar::Type`] from [`GraphQLScalar`] trait.
///
/// [`GraphQLScalar`]: juniper::GraphQLScalar
fn get_scalar(trait_ty: &syn::Path, generics: &syn::Generics) -> scalar::Type {
    if let Some(last_seg) = trait_ty.segments.last() {
        match &last_seg.arguments {
            syn::PathArguments::AngleBracketed(gens) => {
                if let Some(syn::GenericArgument::Type(ty)) = gens.args.last() {
                    let generic_scalar = generics
                        .params
                        .iter()
                        .filter_map(|par| match par {
                            syn::GenericParam::Type(ty) => Some(&ty.ident),
                            _ => None,
                        })
                        .find(|gen_par| gen_par.to_string() == ty.to_token_stream().to_string());

                    return generic_scalar.map_or_else(
                        || scalar::Type::Concrete(ty.clone()),
                        |scalar| scalar::Type::ExplicitGeneric(scalar.clone()),
                    );
                }
            }
            syn::PathArguments::None | syn::PathArguments::Parenthesized(_) => {}
        }
    }
    scalar::Type::Concrete(parse_quote! { ::juniper::DefaultScalarValue })
}

/// Available arguments behind `#[graphql_scalar]` attribute when generating
/// code for [GraphQL scalar][1] type.
///
/// [1]: https://spec.graphql.org/October2021/#sec-Scalars
#[derive(Default)]
struct Attr {
    /// Name of this [GraphQL scalar][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    pub name: Option<SpanContainer<String>>,

    /// Description of this [GraphQL scalar][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    pub description: Option<SpanContainer<String>>,

    /// Spec [`Url`] of this [GraphQL scalar][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    pub specified_by_url: Option<SpanContainer<Url>>,
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

/// Definition of [GraphQL scalar][1] for code generation.
///
/// [1]: https://spec.graphql.org/October2021/#sec-Scalars
struct Definition {
    /// Name of this [GraphQL scalar][1] in GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    name: String,

    /// Rust type that this [GraphQL scalar][1] is represented with.
    ///
    /// It should contain all its generics, if any.
    ///
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    impl_for_type: syn::Type,

    /// Generics of the Rust type that this [GraphQL scalar][1] is implemented
    /// for.
    ///
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    generics: syn::Generics,

    /// Description of this [GraphQL scalar][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    description: Option<String>,

    /// Spec [`Url`] of this [GraphQL scalar][1] to put into GraphQL schema.
    ///
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    specified_by_url: Option<Url>,

    /// [`ScalarValue`] parametrization to generate [`GraphQLType`]
    /// implementation with for this [GraphQL scalar][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [`ScalarValue`]: juniper::ScalarValue
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
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
        self.impl_traits_for_reflection_tokens().to_tokens(into);
    }
}

impl Definition {
    /// Returns generated code implementing [`marker::IsInputType`] and
    /// [`marker::IsOutputType`] trait for this [GraphQL scalar][1].
    ///
    /// [`marker::IsInputType`]: juniper::marker::IsInputType
    /// [`marker::IsOutputType`]: juniper::marker::IsOutputType
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    #[must_use]
    fn impl_output_and_input_type_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::marker::IsInputType<#scalar> for #ty
                #where_clause { }

            impl#impl_gens ::juniper::marker::IsOutputType<#scalar> for #ty
                #where_clause { }
        }
    }

    /// Returns generated code implementing [`GraphQLType`] trait for this
    /// [GraphQL scalar][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    fn impl_type_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let name = &self.name;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
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

    /// Returns generated code implementing [`GraphQLValue`] trait for this
    /// [GraphQL scalar][1].
    ///
    /// [`GraphQLValue`]: juniper::GraphQLValue
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    fn impl_value_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
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
                    selection: Option<&[::juniper::Selection<#scalar>]>,
                    executor: &::juniper::Executor<Self::Context, #scalar>,
                ) -> ::juniper::ExecutionResult<#scalar> {
                    Ok(::juniper::GraphQLScalar::resolve(self))
                }
            }
        }
    }

    /// Returns generated code implementing [`GraphQLValueAsync`] trait for this
    /// [GraphQL scalar][1].
    ///
    /// [`GraphQLValueAsync`]: juniper::GraphQLValueAsync
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    fn impl_value_async_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;

        let generics = self.impl_generics(true);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::GraphQLValueAsync<#scalar> for #ty
                #where_clause
            {
                fn resolve_async<'b>(
                    &'b self,
                    info: &'b Self::TypeInfo,
                    selection_set: Option<&'b [::juniper::Selection<#scalar>]>,
                    executor: &'b ::juniper::Executor<Self::Context, #scalar>,
                ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<#scalar>> {
                    use ::juniper::futures::future;
                    let v = ::juniper::GraphQLValue::resolve(self, info, selection_set, executor);
                    Box::pin(future::ready(v))
                }
            }
        }
    }

    /// Returns generated code implementing [`InputValue`] trait for this
    /// [GraphQL scalar][1].
    ///
    /// [`InputValue`]: juniper::InputValue
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    fn impl_to_input_value_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::ToInputValue<#scalar> for #ty
                #where_clause
            {
                fn to_input_value(&self) -> ::juniper::InputValue<#scalar> {
                    let v = ::juniper::GraphQLScalar::resolve(self);
                    ::juniper::ToInputValue::to_input_value(&v)
                }
            }
        }
    }

    /// Returns generated code implementing [`FromInputValue`] trait for this
    /// [GraphQL scalar][1].
    ///
    /// [`FromInputValue`]: juniper::FromInputValue
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    fn impl_from_input_value_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::FromInputValue<#scalar> for #ty
                #where_clause
            {
                type Error = <Self as ::juniper::GraphQLScalar<#scalar>>::Error;

                fn from_input_value(input: &::juniper::InputValue<#scalar>) -> Result<Self, Self::Error> {
                    ::juniper::GraphQLScalar::from_input_value(input)
                }
            }
        }
    }

    /// Returns generated code implementing [`ParseScalarValue`] trait for this
    /// [GraphQL scalar][1].
    ///
    /// [`ParseScalarValue`]: juniper::ParseScalarValue
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    fn impl_parse_scalar_value_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::ParseScalarValue<#scalar> for #ty
                #where_clause
           {
               fn from_str(
                    token: ::juniper::parser::ScalarToken<'_>,
               ) -> ::juniper::ParseScalarResult<'_, #scalar> {
                    <Self as ::juniper::GraphQLScalar<#scalar>>::from_str(token)
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
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    fn impl_traits_for_reflection_tokens(&self) -> TokenStream {
        let ty = &self.impl_for_type;
        let scalar = &self.scalar;
        let name = &self.name;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::macros::reflection::BaseType<#scalar> for #ty
                #where_clause
            {
                const NAME: ::juniper::macros::reflection::Type = #name;
            }

            impl#impl_gens ::juniper::macros::reflection::BaseSubTypes<#scalar> for #ty
                #where_clause
            {
                const NAMES: ::juniper::macros::reflection::Types =
                    &[<Self as ::juniper::macros::reflection::BaseType<#scalar>>::NAME];
            }

            impl#impl_gens ::juniper::macros::reflection::WrappedType<#scalar> for #ty
                #where_clause
            {
                const VALUE: ::juniper::macros::reflection::WrappedValue = 1;
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
                    lt.lifetime.ident = format_ident!("__fa__{}", ident);
                }

                let lifetimes = generics.lifetimes().map(|lt| &lt.lifetime);
                let ty = &self.impl_for_type;
                let (_, ty_generics, _) = generics.split_for_impl();

                quote! { for<#( #lifetimes ),*> #ty#ty_generics }
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

        generics
    }
}
