//! Code generation for `#[derive(ScalarValue)]` macro.

use std::collections::HashMap;

use proc_macro2::{Literal, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt as _};
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned as _,
    token,
    visit::Visit,
};

use crate::common::{
    diagnostic, filter_attrs,
    parse::{attr::err, ParseBufferExt as _},
    SpanContainer,
};

/// [`diagnostic::Scope`] of errors for `#[derive(ScalarValue)]` macro.
const ERR: diagnostic::Scope = diagnostic::Scope::ScalarValueDerive;

/// Expands `#[derive(ScalarValue)]` macro into generated code.
pub fn expand_derive(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input)?;
    let span = ast.span();

    let data_enum = match ast.data {
        syn::Data::Enum(e) => e,
        _ => return Err(ERR.custom_error(ast.span(), "can only be derived for enums")),
    };

    let attr = Attr::from_attrs("value", &ast.attrs)?;

    let mut methods = HashMap::<Method, Vec<Variant>>::new();
    for var in data_enum.variants.clone() {
        let (ident, field) = (var.ident, Field::try_from(var.fields)?);
        for attr in VariantAttr::from_attrs("value", &var.attrs)?.0 {
            let (method, expr) = attr.into_inner();
            methods.entry(method).or_default().push(Variant {
                ident: ident.clone(),
                field: field.clone(),
                expr,
            });
        }
    }

    let missing_methods = [
        (Method::AsInt, "as_int"),
        (Method::AsFloat, "as_float"),
        (Method::AsStr, "as_str"),
        (Method::AsString, "as_string"),
        (Method::IntoString, "into_string"),
        (Method::AsBool, "as_bool"),
    ]
    .iter()
    .filter_map(|(method, err)| (!methods.contains_key(method)).then_some(err))
    .fold(None, |acc, &method| {
        Some(
            acc.map(|acc| format!("{acc}, {method}"))
                .unwrap_or_else(|| method.into()),
        )
    })
    .filter(|_| !attr.allow_missing_attrs);
    if let Some(missing_methods) = missing_methods {
        return Err(ERR.custom_error(
            span,
            format!(
                "missing `#[value({missing_methods})]` attributes. In case you \
                 are sure that it's ok, use `#[value(allow_missing_attributes)]` \
                 to suppress this error.",
            ),
        ));
    }

    Ok(Definition {
        ident: ast.ident,
        generics: ast.generics,
        variants: data_enum.variants.into_iter().collect(),
        methods,
    }
    .into_token_stream())
}

/// Available arguments behind `#[value]` attribute when generating code for
/// an enum definition.
#[derive(Default)]
struct Attr {
    /// Allows missing [`Method`]s.
    allow_missing_attrs: bool,
}

impl Parse for Attr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Attr> {
        let mut out = Attr::default();
        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;
            match ident.to_string().as_str() {
                "allow_missing_attributes" => {
                    out.allow_missing_attrs = true;
                }
                name => {
                    return Err(err::unknown_arg(&ident, name));
                }
            };
            input.try_parse::<token::Comma>()?;
        }
        Ok(out)
    }
}

impl Attr {
    /// Tries to merge two [`Attr`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(mut self, another: Self) -> syn::Result<Self> {
        self.allow_missing_attrs |= another.allow_missing_attrs;
        Ok(self)
    }

    /// Parses [`Attr`] from the given multiple `name`d [`syn::Attribute`]s
    /// placed on a enum variant.
    fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))
    }
}

/// Possible attribute names of the `#[derive(ScalarValue)]`.
#[derive(Eq, Hash, PartialEq)]
enum Method {
    /// `#[value(as_int)]`.
    AsInt,

    /// `#[value(as_float)]`.
    AsFloat,

    /// `#[value(as_str)]`.
    AsStr,

    /// `#[value(as_string)]`.
    AsString,

    /// `#[value(into_string)]`.
    IntoString,

    /// `#[value(as_bool)]`.
    AsBool,
}

/// Available arguments behind `#[value]` attribute when generating code for an
/// enum variant.
#[derive(Default)]
struct VariantAttr(Vec<SpanContainer<(Method, Option<syn::ExprPath>)>>);

impl Parse for VariantAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<VariantAttr> {
        let mut out = Vec::new();
        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;
            let method = match ident.to_string().as_str() {
                "as_int" => Method::AsInt,
                "as_float" => Method::AsFloat,
                "as_str" => Method::AsStr,
                "as_string" => Method::AsString,
                "into_string" => Method::IntoString,
                "as_bool" => Method::AsBool,
                name => {
                    return Err(err::unknown_arg(&ident, name));
                }
            };
            let expr = input
                .parse::<token::Eq>()
                .ok()
                .map(|_| input.parse::<syn::ExprPath>())
                .transpose()?;
            out.push(SpanContainer::new(
                ident.span(),
                expr.as_ref().map(|e| e.span()),
                (method, expr),
            ));
            input.try_parse::<token::Comma>()?;
        }
        Ok(VariantAttr(out))
    }
}

impl VariantAttr {
    /// Tries to merge two [`VariantAttr`]s into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(mut self, mut another: Self) -> syn::Result<Self> {
        let dup = another.0.iter().find(|m| self.0.contains(m));
        if let Some(dup) = dup {
            Err(err::dup_arg(dup.span_ident()))
        } else {
            self.0.append(&mut another.0);
            Ok(self)
        }
    }

    /// Parses [`VariantAttr`] from the given multiple `name`d
    /// [`syn::Attribute`]s placed on a enum variant.
    fn from_attrs(name: &str, attrs: &[syn::Attribute]) -> syn::Result<Self> {
        filter_attrs(name, attrs)
            .map(|attr| attr.parse_args())
            .try_fold(Self::default(), |prev, curr| prev.try_merge(curr?))
    }
}

/// Definition of a [`ScalarValue`] for code generation.
///
/// [`ScalarValue`]: juniper::ScalarValue
struct Definition {
    /// [`syn::Ident`] of the enum representing this [`ScalarValue`].
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    ident: syn::Ident,

    /// [`syn::Generics`] of the enum representing this [`ScalarValue`].
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    generics: syn::Generics,

    /// [`syn::Variant`]s of the enum representing this [`ScalarValue`].
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    variants: Vec<syn::Variant>,

    /// [`Variant`]s marked with a [`Method`] attribute.
    methods: HashMap<Method, Vec<Variant>>,
}

impl ToTokens for Definition {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.impl_scalar_value_tokens().to_tokens(into);
        self.impl_from_tokens().to_tokens(into);
        self.impl_display_tokens().to_tokens(into);
    }
}

impl Definition {
    /// Returns generated code implementing [`ScalarValue`].
    ///
    /// [`ScalarValue`]: juniper::ScalarValue
    fn impl_scalar_value_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let methods = [
            (
                Method::AsInt,
                quote! { fn as_int(&self) -> ::core::option::Option<::core::primitive::i32> },
                quote! { ::core::primitive::i32::from(*v) },
            ),
            (
                Method::AsFloat,
                quote! { fn as_float(&self) -> ::core::option::Option<::core::primitive::f64> },
                quote! { ::core::primitive::f64::from(*v) },
            ),
            (
                Method::AsStr,
                quote! { fn as_str(&self) -> ::core::option::Option<&::core::primitive::str> },
                quote! { ::core::convert::AsRef::as_ref(v) },
            ),
            (
                Method::AsString,
                quote! { fn as_string(&self) -> ::core::option::Option<::std::string::String> },
                quote! { ::std::string::ToString::to_string(v) },
            ),
            (
                Method::IntoString,
                quote! { fn into_string(self) -> ::core::option::Option<::std::string::String> },
                quote! { ::std::string::String::from(v) },
            ),
            (
                Method::AsBool,
                quote! { fn as_bool(&self) -> ::core::option::Option<::core::primitive::bool> },
                quote! { ::core::primitive::bool::from(*v) },
            ),
        ];
        let methods = methods.iter().map(|(m, sig, def)| {
            let arms = self.methods.get(m).into_iter().flatten().map(|v| {
                let arm = v.match_arm();
                let call = v.expr.as_ref().map_or(def.clone(), |f| quote! { #f(v) });
                quote! { #arm => ::core::option::Option::Some(#call), }
            });
            quote! {
                #sig {
                    match self {
                        #(#arms)*
                        _ => ::core::option::Option::None,
                    }
                }
            }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::juniper::ScalarValue for #ident #ty_gens
                #where_clause
            {
                #( #methods )*
            }
        }
    }

    /// Returns generated code implementing:
    /// - [`From`] each variant into enum itself.
    /// - [`From`] enum into [`Option`] of each variant.
    /// - [`From`] enum reference into [`Option`] of each variant reference.
    fn impl_from_tokens(&self) -> TokenStream {
        let ty_ident = &self.ident;
        let (impl_gen, ty_gen, where_clause) = self.generics.split_for_impl();

        // We don't impose additional bounds on generic parameters, because
        // `ScalarValue` itself has `'static` bound.
        let mut generics = self.generics.clone();
        generics.params.push(parse_quote! { '___a });
        let (lf_impl_gen, _, _) = generics.split_for_impl();

        self.variants
            .iter()
            .map(|v| {
                let var_ident = &v.ident;
                let field = v.fields.iter().next().unwrap();
                let var_ty = &field.ty;
                let var_field = field
                    .ident
                    .as_ref()
                    .map_or_else(|| quote! { (v) }, |i| quote! { { #i: v } });

                quote! {
                    #[automatically_derived]
                    impl #impl_gen ::core::convert::From<#var_ty> for #ty_ident #ty_gen
                        #where_clause
                    {
                        fn from(v: #var_ty) -> Self {
                            Self::#var_ident #var_field
                        }
                    }

                    #[automatically_derived]
                    impl #impl_gen ::core::convert::From<#ty_ident #ty_gen>
                     for ::core::option::Option<#var_ty> #where_clause
                    {
                        fn from(ty: #ty_ident #ty_gen) -> Self {
                            if let #ty_ident::#var_ident #var_field = ty {
                                ::core::option::Option::Some(v)
                            } else {
                                ::core::option::Option::None
                            }
                        }
                    }

                    #[automatically_derived]
                    impl #lf_impl_gen ::core::convert::From<&'___a #ty_ident #ty_gen>
                     for ::core::option::Option<&'___a #var_ty> #where_clause
                    {
                        fn from(ty: &'___a #ty_ident #ty_gen) -> Self {
                            if let #ty_ident::#var_ident #var_field = ty {
                                ::core::option::Option::Some(v)
                            } else {
                                ::core::option::Option::None
                            }
                        }
                    }
                }
            })
            .collect()
    }

    /// Returns generated code implementing [`Display`] by matching over each
    /// enum variant.
    ///
    /// [`Display`]: std::fmt::Display
    fn impl_display_tokens(&self) -> TokenStream {
        let ident = &self.ident;

        let mut generics = self.generics.clone();
        generics.make_where_clause();
        for var in &self.variants {
            let var_ty = &var.fields.iter().next().unwrap().ty;
            let mut check = IsVariantGeneric::new(&self.generics);
            check.visit_type(var_ty);
            if check.res {
                generics
                    .where_clause
                    .as_mut()
                    .unwrap()
                    .predicates
                    .push(parse_quote! { #var_ty: ::core::fmt::Display });
            }
        }
        let (impl_gen, ty_gen, where_clause) = generics.split_for_impl();

        let arms = self.variants.iter().map(|v| {
            let var_ident = &v.ident;
            let field = v.fields.iter().next().unwrap();
            let var_field = field
                .ident
                .as_ref()
                .map_or_else(|| quote! { (v) }, |i| quote! { { #i: v } });

            quote! { Self::#var_ident #var_field => ::core::fmt::Display::fmt(v, f), }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gen ::core::fmt::Display for #ident #ty_gen
                #where_clause
            {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self {
                        #( #arms )*
                    }
                }
            }
        }
    }
}

/// Single-[`Field`] enum variant.
#[derive(Clone)]
struct Variant {
    /// [`Variant`] [`syn::Ident`].
    ident: syn::Ident,

    /// Single [`Variant`] [`Field`].
    field: Field,

    /// Optional resolver provided by [`VariantAttr`].
    expr: Option<syn::ExprPath>,
}

impl Variant {
    /// Returns generated code for matching over this [`Variant`].
    fn match_arm(&self) -> TokenStream {
        let (ident, field) = (&self.ident, &self.field.match_arg());
        quote! {
            Self::#ident #field
        }
    }
}

/// Enum [`Variant`] field.
#[derive(Clone)]
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

impl TryFrom<syn::Fields> for Field {
    type Error = syn::Error;

    fn try_from(value: syn::Fields) -> Result<Self, Self::Error> {
        match value {
            syn::Fields::Named(mut f) if f.named.len() == 1 => {
                Ok(Self::Named(f.named.pop().unwrap().into_value()))
            }
            syn::Fields::Unnamed(mut f) if f.unnamed.len() == 1 => {
                Ok(Self::Unnamed(f.unnamed.pop().unwrap().into_value()))
            }
            _ => Err(ERR.custom_error(value.span(), "expected exactly 1 field")),
        }
    }
}

impl Field {
    /// Returns a [`Field`] for constructing or matching over a [`Variant`].
    fn match_arg(&self) -> TokenStream {
        match self {
            Self::Named(_) => quote! { { #self: v } },
            Self::Unnamed(_) => quote! { (v) },
        }
    }
}

/// [`Visit`]or checking whether a [`Variant`]'s [`Field`] contains generic
/// parameters.
struct IsVariantGeneric<'a> {
    /// Indicates whether the checked [`Variant`]'s [`Field`] contains generic
    /// parameters.
    res: bool,

    /// [`syn::Generics`] to search generic parameters in.
    generics: &'a syn::Generics,
}

impl<'a> IsVariantGeneric<'a> {
    /// Constructs a new [`IsVariantGeneric`] [`Visit`]or.
    fn new(generics: &'a syn::Generics) -> Self {
        Self {
            res: false,
            generics,
        }
    }
}

impl<'ast, 'gen> Visit<'ast> for IsVariantGeneric<'gen> {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        if let Some(ident) = path.get_ident() {
            let is_generic = self.generics.params.iter().any(|par| {
                if let syn::GenericParam::Type(ty) = par {
                    ty.ident == *ident
                } else {
                    false
                }
            });
            if is_generic {
                self.res = true;
            } else {
                syn::visit::visit_path(self, path);
            }
        }
    }
}
