use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token, Data, Fields, Ident, Variant,
};

use crate::{
    common::parse::{
        attr::{err, OptionExt as _},
        ParseBufferExt as _,
    },
    result::GraphQLScope,
    util::{self, filter_attrs, span_container::SpanContainer},
};

#[derive(Debug, Default)]
struct TransparentAttributes {
    transparent: Option<bool>,
    name: Option<String>,
    description: Option<String>,
}

impl syn::parse::Parse for TransparentAttributes {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let mut output = Self {
            transparent: None,
            name: None,
            description: None,
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
                "transparent" => {
                    output.transparent = Some(true);
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
    match ast.data {
        Data::Enum(ref enum_data) => impl_scalar_enum(ast, enum_data, error),
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

    let description = match attrs.description {
        Some(val) => quote!( .description( #val ) ),
        None => quote!(),
    };

    let _async = quote!(
        impl<__S> ::juniper::GraphQLValueAsync<__S> for #ident
        where
            Self: Sync,
            Self::TypeInfo: Sync,
            Self::Context: Sync,
            __S: ::juniper::ScalarValue + Send + Sync,
        {
            fn resolve_async<'a>(
                &'a self,
                info: &'a Self::TypeInfo,
                selection_set: Option<&'a [::juniper::Selection<__S>]>,
                executor: &'a ::juniper::Executor<Self::Context, __S>,
            ) -> ::juniper::BoxFuture<'a, ::juniper::ExecutionResult<__S>> {
                use ::juniper::futures::future;
                let v = ::juniper::GraphQLValue::resolve(self, info, selection_set, executor);
                Box::pin(future::ready(v))
            }
        }
    );

    let content = quote!(
        #_async

        impl<S> ::juniper::GraphQLType<S> for #ident
        where
            S: ::juniper::ScalarValue,
        {
            fn name(_: &Self::TypeInfo) -> Option<&'static str> {
                Some(#name)
            }

            fn meta<'r>(
                info: &Self::TypeInfo,
                registry: &mut ::juniper::Registry<'r, S>,
            ) -> ::juniper::meta::MetaType<'r, S>
            where
                S: 'r,
            {
                registry.build_scalar_type::<Self>(info)
                    #description
                    .into_meta()
            }
        }

        impl<S> ::juniper::GraphQLValue<S> for #ident
        where
            S: ::juniper::ScalarValue,
        {
            type Context = ();
            type TypeInfo = ();

            fn type_name<'__i>(&self, info: &'__i Self::TypeInfo) -> Option<&'__i str> {
                <Self as ::juniper::GraphQLType<S>>::name(info)
            }

            fn resolve(
                &self,
                info: &(),
                selection: Option<&[::juniper::Selection<S>]>,
                executor: &::juniper::Executor<Self::Context, S>,
            ) -> ::juniper::ExecutionResult<S> {
                ::juniper::GraphQLValue::resolve(&self.0, info, selection, executor)
            }
        }

        impl<S> ::juniper::ToInputValue<S> for #ident
        where
            S: ::juniper::ScalarValue,
        {
            fn to_input_value(&self) -> ::juniper::InputValue<S> {
                ::juniper::ToInputValue::to_input_value(&self.0)
            }
        }

        impl<S> ::juniper::FromInputValue<S> for #ident
        where
            S: ::juniper::ScalarValue,
        {
            fn from_input_value(v: &::juniper::InputValue<S>) -> Option<#ident> {
                let inner: #inner_ty = ::juniper::FromInputValue::from_input_value(v)?;
                Some(#ident(inner))
            }
        }

        impl<S> ::juniper::ParseScalarValue<S> for #ident
        where
            S: ::juniper::ScalarValue,
        {
            fn from_str<'a>(
                value: ::juniper::parser::ScalarToken<'a>,
            ) -> ::juniper::ParseScalarResult<'a, S> {
                <#inner_ty as ::juniper::ParseScalarValue<S>>::from_str(value)
            }
        }

        impl<S: ::juniper::ScalarValue> ::juniper::marker::IsOutputType<S> for #ident { }
        impl<S: ::juniper::ScalarValue> ::juniper::marker::IsInputType<S> for #ident { }
    );

    Ok(content)
}

fn impl_scalar_enum(
    ast: &syn::DeriveInput,
    data: &syn::DataEnum,
    error: GraphQLScope,
) -> syn::Result<TokenStream> {
    let ident = &ast.ident;
    let attrs = ScalarValueMeta::from_attrs("graphql", &ast.attrs)?;

    let transparent_impl = if attrs
        .transparent
        .map(SpanContainer::into_inner)
        .unwrap_or(true)
    {
        Some(quote! {
            #[automatically_derived]
            impl ::juniper::TransparentScalarValue for #ident {}
        })
    } else {
        None
    };

    let froms = data
        .variants
        .iter()
        .map(|v| derive_from_variant(v, ident, &error))
        .collect::<Result<Vec<_>, _>>()?;

    let serialize = derive_serialize(data.variants.iter(), ident);

    let display = derive_display(data.variants.iter(), ident);

    Ok(quote! {
        #transparent_impl

        #(#froms)*

        #serialize
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

fn derive_serialize<'a, I>(variants: I, ident: &Ident) -> TokenStream
where
    I: Iterator<Item = &'a Variant>,
{
    let arms = variants.map(|v| {
        let variant = &v.ident;
        quote!(#ident::#variant(ref v) => v.serialize(serializer),)
    });

    quote! {
        impl ::juniper::serde::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
            where S: ::juniper::serde::Serializer
            {
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

/// Available metadata (arguments) behind `#[graphql]` attribute when generating code for
/// `#[derive(GraphQLScalarValue)]` macro placed on enum.
#[derive(Debug, Default)]
struct ScalarValueMeta {
    /// Explicitly specified whether this [`ScalarValue`] allows transparent conversions of
    /// [`FieldError`] type.
    ///
    /// If absent, then it's assumed "yes" by default.
    ///
    /// [`FieldError`]: juniper::FieldError
    /// [`ScalarValue`]: juniper::ScalarValue
    pub transparent: Option<SpanContainer<bool>>,
}

impl Parse for ScalarValueMeta {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut output = Self::default();

        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;
            match ident.to_string().as_str() {
                "transparent" => output
                    .transparent
                    .replace(SpanContainer::new(ident.span(), None, true))
                    .none_or_else(|_| err::dup_arg(&ident))?,
                "not" => {
                    if !input.is_next::<token::Paren>() {
                        return Err(err::unknown_arg(&ident, "not"));
                    }
                    let inner;
                    let _ = syn::parenthesized!(inner in input);
                    let inner_ident = inner.parse::<syn::Ident>()?;
                    match inner_ident.to_string().as_str() {
                        "transparent" => output
                            .transparent
                            .replace(SpanContainer::new(inner_ident.span(), None, false))
                            .none_or_else(|_| err::dup_arg(&inner_ident))?,
                        name => {
                            return Err(err::unknown_arg(&inner_ident, name));
                        }
                    }
                }
                name => {
                    return Err(err::unknown_arg(&ident, name));
                }
            }
            input.try_parse::<token::Comma>()?;
        }

        Ok(output)
    }
}

impl ScalarValueMeta {
    /// Tries to merge two [`ScalarValueMeta`]s into a single one, reporting about duplicates, if
    /// any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            transparent: try_merge_opt!(transparent: self, another),
        })
    }

    /// Parses [`ScalarValueMeta`] from the given multiple `name`d [`syn::Attribute`]s placed on a
    /// type definition.
    pub fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))
    }
}
