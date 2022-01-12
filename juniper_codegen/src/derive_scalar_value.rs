use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    ext::IdentExt as _,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
    token, Data, Fields, Ident, Variant,
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
    util::{self, filter_attrs, get_doc_comment, span_container::SpanContainer},
};

#[derive(Default)]
struct Attr {
    name: Option<SpanContainer<String>>,
    description: Option<SpanContainer<String>>,
    specified_by_url: Option<SpanContainer<Url>>,
    scalar: Option<SpanContainer<scalar::AttrValue>>,
    resolve: Option<SpanContainer<syn::Path>>,
    from_input_value: Option<SpanContainer<syn::Path>>,
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
    field: syn::Field,
    name: String,
    description: Option<String>,
    specified_by_url: Option<Url>,
    scalar: Option<scalar::Type>,
    resolve: Option<syn::Path>,
    from_input_value: Option<syn::Path>,
    from_str: Option<syn::Path>,
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
        let (impl_gens, ty_gens, where_clause) = generics.split_for_impl();

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
        let (impl_gens, ty_gens, where_clause) = generics.split_for_impl();

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
        let field = &self.field.ident;

        let generics = self.impl_generics(false);
        let (impl_gens, ty_gens, where_clause) = generics.split_for_impl();

        let resolve = self.resolve.map_or_else(
            || quote! { ::juniper::GraphQLValue::<#scalar>::resolve(&self.#field, info, selection, executor) },
            |resolve_fn| quote! { Ok(#resolve_fn(self)) },
        );

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
        let (impl_gens, ty_gens, where_clause) = generics.split_for_impl();

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
        let field = &self.field.ident;

        let resolve = self.resolve.map_or_else(
            || quote! { ::juniper::ToInputValue::<#scalar>::to_input_value(self.#field) },
            |resolve_fn| {
                quote! {
                    let v = #resolve_fn(self);
                    ::juniper::ToInputValue::to_input_value(&v)
                }
            },
        );

        let generics = self.impl_generics(false);
        let (impl_gens, ty_gens, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::ToInputValue<#scalar> for #ident#ty_gens
                #where_clause
            {
                fn to_input_value(&self) -> ::juniper::InputValue<#scalar> {
                    #resolve
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
        let field = &self.field.ident;
        let field_ty = &self.field.ty;

        let error_ty = self
            .from_input_value
            .map_or_else(|| quote! { #field_ty }, |_| quote! { Self });
        let from_input_value = self
            .from_input_value
            .map_or_else(
                || quote! { <#field_ty as :juniper::FromInputValue<#scalar>>::from_input_value(&self.#field) },
                |from_input_value_fn| quote! { #from_input_value(self) }
            );

        let generics = self.impl_generics(false);
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        quote! {
            impl#impl_gens ::juniper::FromInputValue<#scalar> for #ty
                #where_clause
            {
                type Error = <#error_ty as ::juniper::GraphQLScalar<#scalar>>::Error;

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
        let field_ty = &self.field.ty;

        let from_str = self.from_str.map_or_else(
            || quote! { <#field_ty as ::juniper::GraphQLScalar<#scalar>>::from_str(token) },
            |from_str_fn| quote! { #from_str_fn(token) },
        );

        let generics = self.impl_generics(false);
        let (impl_gens, ty_gens, where_clause) = generics.split_for_impl();

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

#[derive(Debug, Default)]
struct TransparentAttributes {
    transparent: Option<bool>,
    name: Option<String>,
    description: Option<String>,
    specified_by_url: Option<Url>,
    scalar: Option<syn::Type>,
}

impl syn::parse::Parse for TransparentAttributes {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let mut output = Self {
            transparent: None,
            name: None,
            description: None,
            specified_by_url: None,
            scalar: None,
        };

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<token::Eq>()?;
                    let val = input.parse::<syn::LitStr>()?;
                    output.name = Some(val.value());
                }
                "description" => {
                    input.parse::<token::Eq>()?;
                    let val = input.parse::<syn::LitStr>()?;
                    output.description = Some(val.value());
                }
                "specified_by_url" => {
                    input.parse::<token::Eq>()?;
                    let val: syn::LitStr = input.parse::<syn::LitStr>()?;
                    output.specified_by_url =
                        Some(val.value().parse().map_err(|e| {
                            syn::Error::new(val.span(), format!("Invalid URL: {}", e))
                        })?);
                }
                "transparent" => {
                    output.transparent = Some(true);
                }
                "scalar" | "Scalar" => {
                    input.parse::<token::Eq>()?;
                    let val = input.parse::<syn::Type>()?;
                    output.scalar = Some(val);
                }
                _ => return Err(syn::Error::new(ident.span(), "unknown attribute")),
            }
            input.try_parse::<token::Comma>()?;
        }

        Ok(output)
    }
}

impl TransparentAttributes {
    fn from_attrs(attrs: &[syn::Attribute]) -> syn::parse::Result<Self> {
        match util::find_graphql_attr(attrs) {
            Some(attr) => {
                let mut parsed: TransparentAttributes = attr.parse_args()?;
                if parsed.description.is_none() {
                    parsed.description =
                        util::get_doc_comment(attrs).map(SpanContainer::into_inner);
                }
                Ok(parsed)
            }
            None => Ok(Default::default()),
        }
    }
}

pub fn impl_scalar_value(ast: &syn::DeriveInput, error: GraphQLScope) -> syn::Result<TokenStream> {
    let ident = &ast.ident;

    match ast.data {
        Data::Enum(ref enum_data) => impl_scalar_enum(ident, enum_data, error),
        Data::Struct(ref struct_data) => impl_scalar_struct(ast, struct_data, error),
        Data::Union(_) => Err(error.custom_error(ast.span(), "may not be applied to unions")),
    }
}

fn impl_scalar_struct(
    ast: &syn::DeriveInput,
    data: &syn::DataStruct,
    error: GraphQLScope,
) -> syn::Result<TokenStream> {
    let field = match data.fields {
        syn::Fields::Unnamed(ref fields) if fields.unnamed.len() == 1 => {
            fields.unnamed.first().unwrap()
        }
        _ => {
            return Err(error.custom_error(
                data.fields.span(),
                "requires exact one field, e.g., Test(i32)",
            ))
        }
    };
    let ident = &ast.ident;
    let attrs = TransparentAttributes::from_attrs(&ast.attrs)?;
    let inner_ty = &field.ty;
    let name = attrs.name.unwrap_or_else(|| ident.to_string());

    let description = attrs.description.map(|val| quote!(.description(#val)));
    let specified_by_url = attrs.specified_by_url.map(|url| {
        let url_lit = url.as_str();
        quote!(.specified_by_url(#url_lit))
    });

    let scalar = attrs
        .scalar
        .as_ref()
        .map(|s| quote!( #s ))
        .unwrap_or_else(|| quote!(__S));

    let impl_generics = attrs
        .scalar
        .as_ref()
        .map(|_| quote!())
        .unwrap_or_else(|| quote!(<__S>));

    let _async = quote!(
        impl#impl_generics ::juniper::GraphQLValueAsync<#scalar> for #ident
        where
            Self: Sync,
            Self::TypeInfo: Sync,
            Self::Context: Sync,
            #scalar: ::juniper::ScalarValue + Send + Sync,
        {
            fn resolve_async<'a>(
                &'a self,
                info: &'a Self::TypeInfo,
                selection_set: Option<&'a [::juniper::Selection<#scalar>]>,
                executor: &'a ::juniper::Executor<Self::Context, #scalar>,
            ) -> ::juniper::BoxFuture<'a, ::juniper::ExecutionResult<#scalar>> {
                use ::juniper::futures::future;
                let v = ::juniper::GraphQLValue::<#scalar>::resolve(self, info, selection_set, executor);
                Box::pin(future::ready(v))
            }
        }
    );

    let content = quote!(
        #_async

        impl#impl_generics ::juniper::GraphQLType<#scalar> for #ident
        where
            #scalar: ::juniper::ScalarValue,
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

        impl#impl_generics ::juniper::GraphQLValue<#scalar> for #ident
        where
            #scalar: ::juniper::ScalarValue,
        {
            type Context = ();
            type TypeInfo = ();

            fn type_name<'__i>(&self, info: &'__i Self::TypeInfo) -> Option<&'__i str> {
                <Self as ::juniper::GraphQLType<#scalar>>::name(info)
            }

            fn resolve(
                &self,
                info: &(),
                selection: Option<&[::juniper::Selection<#scalar>]>,
                executor: &::juniper::Executor<Self::Context, #scalar>,
            ) -> ::juniper::ExecutionResult<#scalar> {
                ::juniper::GraphQLValue::<#scalar>::resolve(&self.0, info, selection, executor)
            }
        }

        impl#impl_generics ::juniper::ToInputValue<#scalar> for #ident
        where
            #scalar: ::juniper::ScalarValue,
        {
            fn to_input_value(&self) -> ::juniper::InputValue<#scalar> {
                ::juniper::ToInputValue::<#scalar>::to_input_value(&self.0)
            }
        }

        impl#impl_generics ::juniper::FromInputValue<#scalar> for #ident
        where
            #scalar: ::juniper::ScalarValue,
        {
            type Error = <#inner_ty as ::juniper::FromInputValue<#scalar>>::Error;

            fn from_input_value(
                v: &::juniper::InputValue<#scalar>
            ) -> Result<#ident, <#inner_ty as ::juniper::FromInputValue<#scalar>>::Error> {
                let inner: #inner_ty = ::juniper::FromInputValue::<#scalar>::from_input_value(v)?;
                Ok(#ident(inner))
            }
        }

        impl#impl_generics ::juniper::ParseScalarValue<#scalar> for #ident
        where
            #scalar: ::juniper::ScalarValue,
        {
            fn from_str<'a>(
                value: ::juniper::parser::ScalarToken<'a>,
            ) -> ::juniper::ParseScalarResult<'a, #scalar> {
                <#inner_ty as ::juniper::ParseScalarValue<#scalar>>::from_str(value)
            }
        }

        impl#impl_generics ::juniper::marker::IsOutputType<#scalar> for #ident
            where #scalar: ::juniper::ScalarValue,
        { }
        impl#impl_generics ::juniper::marker::IsInputType<#scalar> for #ident
            where #scalar: ::juniper::ScalarValue,
        { }

        impl#impl_generics ::juniper::macros::reflection::BaseType<#scalar> for #ident
            where #scalar: ::juniper::ScalarValue,
        {
            const NAME: ::juniper::macros::reflection::Type = #name;
        }

        impl#impl_generics ::juniper::macros::reflection::BaseSubTypes<#scalar> for #ident
            where #scalar: ::juniper::ScalarValue,
        {
            const NAMES: ::juniper::macros::reflection::Types =
                &[<Self as ::juniper::macros::reflection::BaseType<#scalar>>::NAME];
        }

        impl#impl_generics ::juniper::macros::reflection::WrappedType<#scalar> for #ident
            where #scalar: ::juniper::ScalarValue,
        {
            const VALUE: ::juniper::macros::reflection::WrappedValue = 1;
        }
    );

    Ok(content)
}

fn impl_scalar_enum(
    ident: &syn::Ident,
    data: &syn::DataEnum,
    error: GraphQLScope,
) -> syn::Result<TokenStream> {
    let froms = data
        .variants
        .iter()
        .map(|v| derive_from_variant(v, ident, &error))
        .collect::<Result<Vec<_>, _>>()?;

    let display = derive_display(data.variants.iter(), ident);

    Ok(quote! {
        #(#froms)*

        #display
    })
}

fn derive_display<'a, I>(variants: I, ident: &Ident) -> TokenStream
where
    I: Iterator<Item = &'a Variant>,
{
    let arms = variants.map(|v| {
        let variant = &v.ident;
        quote!(#ident::#variant(ref v) => write!(f, "{}", v),)
    });

    quote! {
        impl std::fmt::Display for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match *self {
                    #(#arms)*
                }
            }
        }
    }
}

fn derive_from_variant(
    variant: &Variant,
    ident: &Ident,
    error: &GraphQLScope,
) -> syn::Result<TokenStream> {
    let ty = match variant.fields {
        Fields::Unnamed(ref u) if u.unnamed.len() == 1 => &u.unnamed.first().unwrap().ty,

        _ => {
            return Err(error.custom_error(
                variant.fields.span(),
                "requires exact one field, e.g., Test(i32)",
            ))
        }
    };

    let variant = &variant.ident;

    Ok(quote! {
        impl ::std::convert::From<#ty> for #ident {
            fn from(t: #ty) -> Self {
                #ident::#variant(t)
            }
        }

        impl<'a> ::std::convert::From<&'a #ident> for std::option::Option<&'a #ty> {
            fn from(t: &'a #ident) -> Self {
                match *t {
                    #ident::#variant(ref t) => std::option::Option::Some(t),
                    _ => std::option::Option::None
                }
            }
        }

        impl ::std::convert::From<#ident> for std::option::Option<#ty> {
            fn from(t: #ident) -> Self {
                match t {
                    #ident::#variant(t) => std::option::Option::Some(t),
                    _ => std::option::Option::None
                }
            }
        }
    })
}
