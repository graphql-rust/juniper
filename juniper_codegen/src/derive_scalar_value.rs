use proc_macro2::TokenStream;

use quote::quote;
use syn::{self, Data, Fields, Ident, Variant};

use crate::util;

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
                    input.parse::<syn::Token![=]>()?;
                    let val = input.parse::<syn::LitStr>()?;
                    output.name = Some(val.value());
                }
                "description" => {
                    input.parse::<syn::Token![=]>()?;
                    let val = input.parse::<syn::LitStr>()?;
                    output.description = Some(val.value());
                }
                "transparent" => {
                    output.transparent = Some(true);
                }
                other => {
                    return Err(input.error(format!("Unknown attribute: {}", other)));
                }
            }
            if input.lookahead1().peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
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
                    parsed.description = util::get_doc_comment(attrs);
                }
                Ok(parsed)
            }
            None => Ok(Default::default()),
        }
    }
}

pub fn impl_scalar_value(ast: &syn::DeriveInput, is_internal: bool) -> TokenStream {
    let ident = &ast.ident;

    match ast.data {
        Data::Enum(ref enum_data) => impl_scalar_enum(ident, enum_data, is_internal),
        Data::Struct(ref struct_data) => impl_scalar_struct(ast, struct_data, is_internal),
        Data::Union(_) => {
            panic!("#[derive(GraphQLScalarValue)] may not be applied to unions");
        }
    }
}

fn impl_scalar_struct(
    ast: &syn::DeriveInput,
    data: &syn::DataStruct,
    is_internal: bool,
) -> TokenStream {
    let field = match data.fields {
        syn::Fields::Unnamed(ref fields) if fields.unnamed.len() == 1 => {
            fields.unnamed.first().unwrap()
        }
        _ => {
            panic!("#[derive(GraphQLScalarValue)] may only be applied to enums or tuple structs with a single field");
        }
    };
    let ident = &ast.ident;
    let attrs = match TransparentAttributes::from_attrs(&ast.attrs) {
        Ok(attrs) => attrs,
        Err(e) => {
            panic!("Invalid #[graphql] attribute: {}", e);
        }
    };
    let inner_ty = &field.ty;
    let name = attrs.name.unwrap_or_else(|| ident.to_string());

    let crate_name = if is_internal {
        quote!(crate)
    } else {
        quote!(juniper)
    };

    let description = match attrs.description {
        Some(val) => quote!( .description( #val ) ),
        None => quote!(),
    };

    let _async = quote!(

        impl <__S> #crate_name::GraphQLTypeAsync<__S> for #ident
        where
            __S: #crate_name::ScalarValue + Send + Sync,
            Self: #crate_name::GraphQLType<__S> + Send + Sync,
            Self::Context: Send + Sync,
            Self::TypeInfo: Send + Sync,
        {
            fn resolve_async<'a>(
                &'a self,
                info: &'a Self::TypeInfo,
                selection_set: Option<&'a [#crate_name::Selection<__S>]>,
                executor: &'a #crate_name::Executor<Self::Context, __S>,
            ) -> #crate_name::BoxFuture<'a, #crate_name::ExecutionResult<__S>> {
                use #crate_name::GraphQLType;
                use futures::future;
                let v = self.resolve(info, selection_set, executor);
                Box::pin(future::ready(v))
            }
        }
    );

    quote!(
        #_async

        impl<S> #crate_name::GraphQLType<S> for #ident
        where
            S: #crate_name::ScalarValue,
        {
            type Context = ();
            type TypeInfo = ();

            fn name(_: &Self::TypeInfo) -> Option<&str> {
                Some(#name)
            }

            fn meta<'r>(
                info: &Self::TypeInfo,
                registry: &mut #crate_name::Registry<'r, S>,
            ) -> #crate_name::meta::MetaType<'r, S>
            where
                S: 'r,
            {
                registry.build_scalar_type::<Self>(info)
                    #description
                    .into_meta()
            }

            fn resolve(
                &self,
                info: &(),
                selection: Option<&[#crate_name::Selection<S>]>,
                executor: &#crate_name::Executor<Self::Context, S>,
            ) -> #crate_name::ExecutionResult<S> {
                #crate_name::GraphQLType::resolve(&self.0, info, selection, executor)
            }
        }

        impl<S> #crate_name::ToInputValue<S> for #ident
        where
            S: #crate_name::ScalarValue,
        {
            fn to_input_value(&self) -> #crate_name::InputValue<S> {
                #crate_name::ToInputValue::to_input_value(&self.0)
            }
        }

        impl<S> #crate_name::FromInputValue<S> for #ident
        where
            S: #crate_name::ScalarValue,
        {
            fn from_input_value(v: &#crate_name::InputValue<S>) -> Option<#ident> {
                let inner: #inner_ty = #crate_name::FromInputValue::from_input_value(v)?;
                Some(#ident(inner))
            }
        }

        impl<S> #crate_name::ParseScalarValue<S> for #ident
        where
            S: #crate_name::ScalarValue,
        {
            fn from_str<'a>(
                value: #crate_name::parser::ScalarToken<'a>,
            ) -> #crate_name::ParseScalarResult<'a, S> {
                <#inner_ty as #crate_name::ParseScalarValue<S>>::from_str(value)
            }
        }
    )
}

fn impl_scalar_enum(ident: &syn::Ident, data: &syn::DataEnum, is_internal: bool) -> TokenStream {
    let froms = data
        .variants
        .iter()
        .map(|v| derive_from_variant(v, ident))
        .collect::<Result<Vec<_>, String>>()
        .unwrap_or_else(|s| panic!("{}", s));

    let serialize = derive_serialize(data.variants.iter(), ident, is_internal);

    let display = derive_display(data.variants.iter(), ident);

    quote! {
        #(#froms)*

        #serialize
        #display
    }
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

fn derive_serialize<'a, I>(variants: I, ident: &Ident, is_internal: bool) -> TokenStream
where
    I: Iterator<Item = &'a Variant>,
{
    let arms = variants.map(|v| {
        let variant = &v.ident;
        quote!(#ident::#variant(ref v) => v.serialize(serializer),)
    });

    let serde_path = if is_internal {
        quote!(crate::serde)
    } else {
        quote!(juniper::serde)
    };

    quote! {
        impl #serde_path::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where S: #serde_path::Serializer
            {
                match *self {
                    #(#arms)*
                }
            }
        }
    }
}

fn derive_from_variant(variant: &Variant, ident: &Ident) -> Result<TokenStream, String> {
    let ty = match variant.fields {
        Fields::Unnamed(ref u) if u.unnamed.len() == 1 => &u.unnamed.first().unwrap().ty,

        _ => {
            return Err(String::from(
                "Only enums with exactly one unnamed field per variant are supported",
            ));
        }
    };

    let variant = &variant.ident;

    Ok(quote! {
        impl std::convert::From<#ty> for #ident {
            fn from(t: #ty) -> Self {
                #ident::#variant(t)
            }
        }

        impl<'a> std::convert::From<&'a #ident> for std::option::Option<&'a #ty> {
            fn from(t: &'a #ident) -> Self {
                match *t {
                    #ident::#variant(ref t) => std::option::Option::Some(t),
                    _ => std::option::Option::None
                }
            }
        }

        impl std::convert::From<#ident> for std::option::Option<#ty> {
            fn from(t: #ident) -> Self {
                match t {
                    #ident::#variant(t) => std::option::Option::Some(t),
                    _ => std::option::Option::None
                }
            }
        }
    })
}
