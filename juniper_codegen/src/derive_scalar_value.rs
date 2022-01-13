use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{
    ext::IdentExt as _,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
    token,
};
use url::Url;

use crate::{
    common::{
        parse::{
            attr::{err, OptionExt},
            ParseBufferExt as _,
        },
        scalar,
    },
    result::GraphQLScope,
    util::{filter_attrs, get_doc_comment, span_container::SpanContainer},
};

const ERR: GraphQLScope = GraphQLScope::DeriveScalar;

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input)?;

    let attr = Attr::from_attrs("graphql", &ast.attrs)?;

    let field = match (
        attr.resolve.as_deref().cloned(),
        attr.from_input_value.as_deref().cloned(),
        attr.from_input_value_err.as_deref().cloned(),
        attr.from_str.as_deref().cloned(),
    ) {
        (Some(resolve), Some(from_input_value), Some(from_input_value_err), Some(from_str)) => {
            GraphQLScalarDefinition::Custom {
                resolve,
                from_input_value: (from_input_value, from_input_value_err),
                from_str,
            }
        }
        (resolve, from_input_value, from_input_value_err, from_str) => {
            let from_input_value = match (from_input_value, from_input_value_err) {
                (Some(from_input_value), Some(err)) => Some((from_input_value, err)),
                (None, None) => None,
                _ => {
                    return Err(ERR.custom_error(
                        ast.span(),
                        "`from_input_value` attribute should be provided in \
                     tandem with `from_input_value_err`",
                    ))
                }
            };

            let data = if let syn::Data::Struct(data) = &ast.data {
                data
            } else {
                return Err(ERR.custom_error(
                    ast.span(),
                    "expected single-field struct \
                      or all `resolve`, `from_input_value` and `from_str` functions",
                ));
            };
            let field = match &data.fields {
                syn::Fields::Unit => Err(ERR.custom_error(
                    ast.span(),
                    "expected exactly 1 field, e.g., `Test(i32)` or `Test { test: i32 }` \
                     or all `resolve`, `from_input_value` and `from_str` functions",
                )),
                syn::Fields::Unnamed(fields) => fields
                    .unnamed
                    .first()
                    .and_then(|f| (fields.unnamed.len() == 1).then(|| Field::Unnamed(f.clone())))
                    .ok_or_else(|| {
                        ERR.custom_error(
                            ast.span(),
                            "expected exactly 1 field, e.g., Test(i32)\
                             or all `resolve`, `from_input_value` and `from_str` functions",
                        )
                    }),
                syn::Fields::Named(fields) => fields
                    .named
                    .first()
                    .and_then(|f| (fields.named.len() == 1).then(|| Field::Named(f.clone())))
                    .ok_or_else(|| {
                        ERR.custom_error(
                            ast.span(),
                            "expected exactly 1 field, e.g., Test { test: i32 }\
                             or all `resolve`, `from_input_value` and `from_str` functions",
                        )
                    }),
            }?;
            GraphQLScalarDefinition::Delegated {
                resolve,
                from_input_value,
                from_str,
                field,
            }
        }
    };

    let scalar = scalar::Type::parse(attr.scalar.as_deref(), &ast.generics);

    Ok(Definition {
        ident: ast.ident.clone(),
        generics: ast.generics.clone(),
        field,
        name: attr
            .name
            .as_deref()
            .cloned()
            .unwrap_or_else(|| ast.ident.to_string()),
        description: attr.description.as_deref().cloned(),
        specified_by_url: attr.specified_by_url.as_deref().cloned(),
        scalar,
    }
    .to_token_stream())
}

enum GraphQLScalarDefinition {
    Custom {
        resolve: syn::Path,
        from_input_value: (syn::Path, syn::Type),
        from_str: syn::Path,
    },
    Delegated {
        resolve: Option<syn::Path>,
        from_input_value: Option<(syn::Path, syn::Type)>,
        from_str: Option<syn::Path>,
        field: Field,
    },
}

impl GraphQLScalarDefinition {
    fn resolve(&self, scalar: &scalar::Type) -> TokenStream {
        match self {
            Self::Custom { resolve, .. }
            | Self::Delegated {
                resolve: Some(resolve),
                ..
            } => {
                quote! { Ok(#resolve(self)) }
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

    fn to_input_value(&self, scalar: &scalar::Type) -> TokenStream {
        match self {
            Self::Custom { resolve, .. }
            | Self::Delegated {
                resolve: Some(resolve),
                ..
            } => {
                quote! {
                    let v = #resolve(self);
                    ::juniper::ToInputValue::to_input_value(&v)
                }
            }
            Self::Delegated { field, .. } => {
                quote! { ::juniper::ToInputValue::<#scalar>::to_input_value(&self.#field) }
            }
        }
    }

    fn from_input_value_err(&self, scalar: &scalar::Type) -> TokenStream {
        match self {
            Self::Custom {
                from_input_value: (_, err),
                ..
            }
            | Self::Delegated {
                from_input_value: Some((_, err)),
                ..
            } => quote! { #err },
            Self::Delegated { field, .. } => {
                let field_ty = field.ty();
                quote! { <#field_ty as ::juniper::GraphQLScalar<#scalar>>::Error }
            }
        }
    }

    fn from_input_value(&self, scalar: &scalar::Type) -> TokenStream {
        match self {
            Self::Custom {
                from_input_value: (from_input_value, _),
                ..
            }
            | Self::Delegated {
                from_input_value: Some((from_input_value, _)),
                ..
            } => {
                quote! { #from_input_value(input) }
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

    fn from_str(&self, scalar: &scalar::Type) -> TokenStream {
        match self {
            Self::Custom { from_str, .. }
            | Self::Delegated {
                from_str: Some(from_str),
                ..
            } => {
                quote! { #from_str(token) }
            }
            Self::Delegated { field, .. } => {
                let field_ty = field.ty();
                quote! { <#field_ty as ::juniper::GraphQLScalar<#scalar>>::from_str(token) }
            }
        }
    }
}

enum Field {
    Named(syn::Field),
    Unnamed(syn::Field),
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Field::Named(f) => f.ident.to_tokens(tokens),
            Field::Unnamed(_) => tokens.append(Literal::u8_unsuffixed(0)),
        }
    }
}

impl Field {
    fn ty(&self) -> &syn::Type {
        match self {
            Field::Named(f) | Field::Unnamed(f) => &f.ty,
        }
    }

    fn closure_constructor(&self) -> TokenStream {
        match self {
            Field::Named(syn::Field { ident, .. }) => {
                quote! { |v| Self { #ident: v } }
            }
            Field::Unnamed(_) => quote! { Self },
        }
    }
}

#[derive(Default)]
struct Attr {
    name: Option<SpanContainer<String>>,
    description: Option<SpanContainer<String>>,
    specified_by_url: Option<SpanContainer<Url>>,
    scalar: Option<SpanContainer<scalar::AttrValue>>,
    resolve: Option<SpanContainer<syn::Path>>,
    from_input_value: Option<SpanContainer<syn::Path>>,
    from_input_value_err: Option<SpanContainer<syn::Type>>,
    from_str: Option<SpanContainer<syn::Path>>,
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
                "resolve" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<syn::Path>()?;
                    out.resolve
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "from_input_value" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<syn::Path>()?;
                    out.from_input_value
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "from_input_value_err" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<syn::Type>()?;
                    out.from_input_value_err
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| err::dup_arg(&ident))?
                }
                "from_str" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<syn::Path>()?;
                    out.from_str
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
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
            scalar: try_merge_opt!(scalar: self, another),
            resolve: try_merge_opt!(resolve: self, another),
            from_input_value: try_merge_opt!(from_input_value: self, another),
            from_input_value_err: try_merge_opt!(from_input_value_err: self, another),
            from_str: try_merge_opt!(from_str: self, another),
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

struct Definition {
    ident: syn::Ident,
    generics: syn::Generics,
    field: GraphQLScalarDefinition,
    name: String,
    description: Option<String>,
    specified_by_url: Option<Url>,
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
        let ident = &self.ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();
        let (_, ty_gens, _) = self.generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::marker::IsInputType<#scalar> for #ident#ty_gens
                #where_clause { }

            impl#impl_gens ::juniper::marker::IsOutputType<#scalar> for #ident#ty_gens
                #where_clause { }
        }
    }

    /// Returns generated code implementing [`GraphQLType`] trait for this
    /// [GraphQL scalar][1].
    ///
    /// [`GraphQLType`]: juniper::GraphQLType
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    fn impl_type_tokens(&self) -> TokenStream {
        let ident = &self.ident;
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

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();
        let (_, ty_gens, _) = self.generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::GraphQLType<#scalar> for #ident#ty_gens
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
        let ident = &self.ident;
        let scalar = &self.scalar;

        let resolve = self.field.resolve(&scalar);

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();
        let (_, ty_gens, _) = self.generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::GraphQLValue<#scalar> for #ident#ty_gens
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
                    #resolve
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
        let ident = &self.ident;
        let scalar = &self.scalar;

        let generics = self.impl_generics(true);
        let (impl_gens, _, where_clause) = generics.split_for_impl();
        let (_, ty_gens, _) = self.generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::GraphQLValueAsync<#scalar> for #ident#ty_gens
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
        let ident = &self.ident;
        let scalar = &self.scalar;

        let to_input_value = self.field.to_input_value(&scalar);

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();
        let (_, ty_gens, _) = self.generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::ToInputValue<#scalar> for #ident#ty_gens
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
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    fn impl_from_input_value_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;

        let error_ty = self.field.from_input_value_err(&scalar);
        let from_input_value = self.field.from_input_value(&scalar);

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();
        let (_, ty_gens, _) = self.generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::FromInputValue<#scalar> for #ident#ty_gens
                #where_clause
            {
                type Error = #error_ty;

                fn from_input_value(input: &::juniper::InputValue<#scalar>) -> Result<Self, Self::Error> {
                   #from_input_value
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
        let ident = &self.ident;
        let scalar = &self.scalar;

        let from_str = self.field.from_str(&scalar);

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();
        let (_, ty_gens, _) = self.generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::ParseScalarValue<#scalar> for #ident#ty_gens
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

    /// Returns generated code implementing [`BaseType`], [`BaseSubTypes`] and
    /// [`WrappedType`] traits for this [GraphQL scalar][1].
    ///
    /// [`BaseSubTypes`]: juniper::macros::reflection::BaseSubTypes
    /// [`BaseType`]: juniper::macros::reflection::BaseType
    /// [`WrappedType`]: juniper::macros::reflection::WrappedType
    /// [1]: https://spec.graphql.org/October2021/#sec-Scalars
    fn impl_traits_for_reflection_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let scalar = &self.scalar;
        let name = &self.name;

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();
        let (_, ty_gens, _) = self.generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::macros::reflection::BaseType<#scalar> for #ident#ty_gens
                #where_clause
            {
                const NAME: ::juniper::macros::reflection::Type = #name;
            }

            impl#impl_gens ::juniper::macros::reflection::BaseSubTypes<#scalar> for #ident#ty_gens
                #where_clause
            {
                const NAMES: ::juniper::macros::reflection::Types =
                    &[<Self as ::juniper::macros::reflection::BaseType<#scalar>>::NAME];
            }

            impl#impl_gens ::juniper::macros::reflection::WrappedType<#scalar> for #ident#ty_gens
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
                let ty = &self.ident;
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
