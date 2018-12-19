use proc_macro2::TokenStream;

use syn::{self, Data, Fields, Ident, Variant};

pub fn impl_scalar_value(ast: &syn::DeriveInput, is_internal: bool) -> TokenStream {
    let ident = &ast.ident;

    let variants = match ast.data {
        Data::Enum(ref enum_data) => &enum_data.variants,
        _ => {
            panic!("#[derive(GraphQLScalarValue)] may only be applied to enums, not to structs");
        }
    };

    let froms = variants
        .iter()
        .map(|v| derive_from_variant(v, ident))
        .collect::<Result<Vec<_>, String>>()
        .unwrap_or_else(|s| panic!("{}", s));

    let serialize = derive_serialize(variants.iter(), ident, is_internal);

    let display = derive_display(variants.iter(), ident);

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
        Fields::Unnamed(ref u) if u.unnamed.len() == 1 => &u.unnamed.first().unwrap().value().ty,

        _ => {
            return Err(String::from(
                "Only enums with exactly one unnamed field per variant are supported",
            ))
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
