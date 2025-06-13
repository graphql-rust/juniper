//! Code generation for `#[derive(ScalarValue)]` macro.

use std::collections::HashMap;

use proc_macro2::{Literal, TokenStream};
use quote::{ToTokens, TokenStreamExt as _, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned as _,
    token,
};

use crate::common::{
    SpanContainer, diagnostic, filter_attrs,
    parse::{
        ParseBufferExt as _,
        attr::{OptionExt as _, err},
    },
};

/// [`diagnostic::Scope`] of errors for `#[derive(ScalarValue)]` macro.
const ERR: diagnostic::Scope = diagnostic::Scope::ScalarValueDerive;

/// Expands `#[derive(ScalarValue)]` macro into generated code.
pub fn expand_derive(input: TokenStream) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input)?;

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

    Ok(Definition {
        ident: ast.ident,
        generics: ast.generics,
        variants: data_enum.variants.into_iter().collect(),
        methods,
        from_displayable: attr.from_displayable.map(SpanContainer::into_inner),
    }
    .into_token_stream())
}

/// Available arguments behind `#[value]` attribute when generating code for
/// an enum definition.
#[derive(Default)]
struct Attr {
    /// Explicitly specified function to be used as `ScalarValue::from_displayable()`
    /// implementation.
    from_displayable: Option<SpanContainer<syn::ExprPath>>,
}

impl Parse for Attr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Attr> {
        let mut out = Attr::default();
        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;
            match ident.to_string().as_str() {
                "from_displayable_with" => {
                    input.parse::<token::Eq>()?;
                    let scl = input.parse::<syn::ExprPath>()?;
                    out.from_displayable
                        .replace(SpanContainer::new(ident.span(), Some(scl.span()), scl))
                        .none_or_else(|_| err::dup_arg(&ident))?
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
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            from_displayable: try_merge_opt!(from_displayable: self, another),
        })
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
    /// `#[value(to_int)]`.
    ToInt,

    /// `#[value(to_float)]`.
    ToFloat,

    /// `#[value(as_str)]`.
    AsStr,

    /// `#[value(to_string)]`.
    ToString,

    /// `#[value(to_bool)]`.
    ToBool,
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
                "to_int" => Method::ToInt,
                "to_float" => Method::ToFloat,
                "as_str" => Method::AsStr,
                "to_string" => Method::ToString,
                "to_bool" => Method::ToBool,
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

/// Definition of a `ScalarValue` for code generation.
struct Definition {
    /// [`syn::Ident`] of the enum representing this `ScalarValue`.
    ident: syn::Ident,

    /// [`syn::Generics`] of the enum representing this `ScalarValue`.
    generics: syn::Generics,

    /// [`syn::Variant`]s of the enum representing this `ScalarValue`.
    variants: Vec<syn::Variant>,

    /// [`Variant`]s marked with a [`Method`] attribute.
    methods: HashMap<Method, Vec<Variant>>,

    /// Custom definition to call in `ScalarValue::from_displayable()` method.
    ///
    /// If [`None`] then `ScalarValue::from_displayable()` method is not generated.
    from_displayable: Option<syn::ExprPath>,
}

impl ToTokens for Definition {
    fn to_tokens(&self, into: &mut TokenStream) {
        self.impl_scalar_value_tokens().to_tokens(into);
        self.impl_try_scalar_to_tokens().to_tokens(into);
    }
}

impl Definition {
    /// Returns generated code implementing `ScalarValue`.
    fn impl_scalar_value_tokens(&self) -> TokenStream {
        let ty_ident = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let is_type = {
            let arms = self.variants.iter().map(|var| {
                let var_ident = &var.ident;
                let field = Field::try_from(var.fields.clone())
                    .unwrap_or_else(|_| unreachable!("already checked"));
                let var_pattern = field.match_arg();

                quote! {
                    Self::#var_ident #var_pattern => ::juniper::AnyExt::is::<__T>(v),
                }
            });

            quote! {
                fn is_type<__T: ::core::any::Any + ?::core::marker::Sized>(&self) -> bool {
                    match self {
                        #( #arms )*
                    }
                }
            }
        };

        let from_displayable = self.from_displayable.as_ref().map(|expr| {
            quote! {
                fn from_displayable<
                    __T: ::core::fmt::Display + ::core::any::Any + ?::core::marker::Sized,
                >(__v: &__T) -> Self {
                    #expr(__v)
                }
            }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::juniper::ScalarValue for #ty_ident #ty_gens #where_clause {
                #is_type
                #from_displayable
            }
        }
    }

    /// Returns generated code implementing `TryScalarValueTo`.
    fn impl_try_scalar_to_tokens(&self) -> TokenStream {
        let ty_ident = &self.ident;
        let (_, ty_gens, where_clause) = self.generics.split_for_impl();

        let ref_lt = quote! { '___a };
        // We don't impose additional bounds on generic parameters, because
        // `ScalarValue` itself has `'static` bound.
        let mut generics = self.generics.clone();
        generics.params.push(parse_quote! { #ref_lt });
        let (lt_impl_gens, _, _) = generics.split_for_impl();

        let methods = [
            (
                Method::ToInt,
                "Int",
                quote! { ::core::primitive::i32 },
                quote! { ::core::convert::Into::into(*v) },
            ),
            (
                Method::ToFloat,
                "Float",
                quote! { ::core::primitive::f64 },
                quote! { ::core::convert::Into::into(*v) },
            ),
            (
                Method::AsStr,
                "String",
                quote! { &#ref_lt ::core::primitive::str },
                quote! { ::core::convert::AsRef::as_ref(v) },
            ),
            (
                Method::ToString,
                "String",
                quote! { ::std::string::String },
                quote! { ::std::string::ToString::to_string(v) },
            ),
            (
                Method::ToBool,
                "Bool",
                quote! { ::core::primitive::bool },
                quote! { ::core::convert::Into::into(*v) },
            ),
        ];
        let impls = methods.iter().filter_map(|(m, into_name, as_ty, default_expr)| {
            let arms = self.methods.get(m)?.iter().map(|v| {
                let arm_pattern = v.match_arm();
                let call = if let Some(func) = &v.expr {
                    quote! { #func(v) }
                } else {
                    default_expr.clone()
                };
                quote! {
                    #arm_pattern => ::core::result::Result::Ok(#call),
                }
            });
            Some(quote! {
                #[automatically_derived]
                impl #lt_impl_gens ::juniper::TryScalarValueTo<#ref_lt, #as_ty>
                 for #ty_ident #ty_gens #where_clause
                {
                    type Error = ::juniper::WrongInputScalarTypeError<#ref_lt, #ty_ident #ty_gens>;

                    fn try_scalar_value_to(
                        &#ref_lt self,
                    ) -> ::core::result::Result<#as_ty, Self::Error> {
                        match self {
                            #( #arms )*
                            _ => ::core::result::Result::Err(::juniper::WrongInputScalarTypeError {
                                type_name: ::juniper::arcstr::literal!(#into_name),
                                input: self,
                            }),
                        }
                    }
                }
            })
        });
        quote! {
            #( #impls )*
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
    Named(Box<syn::Field>),

    /// Unnamed [`Field`].
    Unnamed,
}

impl ToTokens for Field {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Named(f) => f.ident.to_tokens(tokens),
            Self::Unnamed => tokens.append(Literal::u8_unsuffixed(0)),
        }
    }
}

impl TryFrom<syn::Fields> for Field {
    type Error = syn::Error;

    fn try_from(value: syn::Fields) -> Result<Self, Self::Error> {
        match value {
            syn::Fields::Named(mut f) if f.named.len() == 1 => {
                Ok(Self::Named(Box::new(f.named.pop().unwrap().into_value())))
            }
            syn::Fields::Unnamed(f) if f.unnamed.len() == 1 => Ok(Self::Unnamed),
            _ => Err(ERR.custom_error(value.span(), "expected exactly 1 field")),
        }
    }
}

impl Field {
    /// Returns a [`Field`] for constructing or matching over a [`Variant`].
    fn match_arg(&self) -> TokenStream {
        match self {
            Self::Named(_) => quote! { { #self: v } },
            Self::Unnamed => quote! { (v) },
        }
    }
}
