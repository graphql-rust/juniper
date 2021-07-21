use std::{collections::HashMap, mem, str::FromStr};

use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned as _,
    token,
};

#[derive(Debug, Default)]
pub struct Attr {
    name: Option<syn::LitStr>,
    level: Option<syn::LitStr>,
    target: Option<syn::LitStr>,
    skip: HashMap<String, syn::Ident>,
    fields: Vec<syn::ExprAssign>,
    is_complex: bool,
    no_trace: bool,
}

impl Attr {
    /// Parses [`TracingAttr`] from `method`s attributes and removes itself from
    /// `method.attrs` if present.
    pub fn from_method(method: &mut syn::TraitItemMethod) -> Option<Self> {
        let attr = method
            .attrs
            .iter()
            .find(|attr| attr.path.is_ident("tracing"))
            .map(|attr| attr.parse_args())
            .transpose();

        method.attrs = mem::take(&mut method.attrs)
            .into_iter()
            .filter(|attr| !attr.path.is_ident("tracing"))
            .collect();

        match attr {
            Ok(attr) => attr,
            Err(e) => abort!(e),
        }
    }
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attr = Attr::default();

        while !input.is_empty() {
            let name = input.parse::<syn::Ident>()?;

            match name.to_string().as_str() {
                "name" => {
                    input.parse::<token::Eq>()?;
                    attr.name = Some(input.parse()?);
                }
                "level" => {
                    input.parse::<token::Eq>()?;
                    attr.level = Some(input.parse()?);
                }
                "target" => {
                    input.parse::<token::Eq>()?;
                    attr.target = Some(input.parse()?);
                }
                "skip" => {
                    let skipped_fields;
                    syn::parenthesized!(skipped_fields in input);
                    while !skipped_fields.is_empty() {
                        let field: syn::Ident = skipped_fields.parse()?;
                        attr.skip.insert(field.to_string(), field);

                        skipped_fields.parse::<token::Comma>().ok();
                    }
                }
                "no_trace" => {
                    attr.no_trace = true;
                }
                "complex" => {
                    attr.is_complex = true;
                }
                "fields" => {
                    let fields;
                    syn::parenthesized!(fields in input);
                    while !fields.is_empty() {
                        attr.fields.push(fields.parse()?);

                        fields.parse::<token::Comma>().ok();
                    }
                }
                _ => return Err(syn::Error::new(name.span(), "unknown attribute")),
            }

            // Discard trailing comma.
            input.parse::<token::Comma>().ok();
        }
        Ok(attr)
    }
}

/// The different possible groups of fields to trace.
#[derive(Copy, Clone, Debug)]
pub enum TracingRule {
    /// Trace all fields that resolved using `async fn`s.
    Async,

    /// Trace all fields that can be syncronously resolved.
    Sync,

    /// Trace only fields that marked with `#[tracing(complex)]`.
    Complex,

    /// Skip tracing of all fields.
    SkipAll,
}

impl TracingRule {
    #[cfg(any(feature = "trace-async", feature = "trace-sync"))]
    pub fn is_traced(&self, field: &impl TracedField) -> bool {
        match self {
            Self::Async => field.is_async(),
            Self::Sync => !field.is_async(),
            Self::Complex => field.tracing_attr().map_or(false, |t| t.is_complex),
            Self::SkipAll => false,
        }
    }
}

impl FromStr for TracingRule {
    type Err = ();

    fn from_str(rule: &str) -> Result<Self, Self::Err> {
        match rule {
            "trace-async" => Ok(Self::Async),
            "trace-sync" => Ok(Self::Sync),
            "skip-all" => Ok(Self::SkipAll),
            "complex" => Ok(Self::Complex),
            _ => Err(()),
        }
    }
}

pub trait TracedType {
    fn tracing_rule(&self) -> Option<TracingRule>;
    fn name(&self) -> &str;
    fn scalar(&self) -> Option<syn::Type>;
}

pub trait TracedField {
    type Arg: TracedArgument;

    fn tracing_attr(&self) -> Option<&Attr>;
    fn is_async(&self) -> bool;
    fn name(&self) -> &str;
    fn args(&self) -> Vec<&Self::Arg>;
}

pub trait TracedArgument {
    fn ty(&self) -> &syn::Type;
    fn name(&self) -> &str;
}

#[cfg(any(feature = "trace-async", feature = "trace-sync"))]
fn is_traced(ty: &impl TracedType, field: &impl TracedField) -> bool {
    let traced = ty
        .tracing_rule()
        .map_or_else(|| true, |rule| rule.is_traced(field));

    let no_trace = field.tracing_attr().map(|t| t.no_trace).unwrap_or(false);

    traced && !no_trace
}

#[cfg(not(any(feature = "trace-async", feature = "trace-sync")))]
fn is_traced(_: &impl TracedType, _: &impl TracedField) -> bool {
    false
}

#[cfg(any(feature = "trace-async", feature = "trace-sync"))]
pub fn instrument() -> TokenStream {
    quote!(
        use ::juniper::InstrumentInternal as _;
    )
}

#[cfg(not(any(feature = "trace-async", feature = "trace-sync")))]
pub fn instrument() -> TokenStream {
    quote!()
}

// Returns code that constructs `span` required for tracing
pub fn span_tokens(ty: &impl TracedType, field: &impl TracedField) -> TokenStream {
    if !is_traced(ty, field) {
        return quote!();
    }

    let name = field.name();
    let span_name = format!("{}.{}", ty.name(), name);
    let span_name = syn::LitStr::new(&span_name, name.span());

    let args = field.args().into_iter().filter_map(|arg| {
        let name = arg.name();
        let arg_name = syn::LitStr::new(name, arg.ty().span());
        let arg_getter = syn::LitStr::new(name, arg.ty().span());
        let scalar = &ty
            .scalar()
            .unwrap_or_else(|| syn::parse2(quote!(::juniper::DefaultScalarValue)).unwrap());
        let ty = arg.ty();

        field
            .tracing_attr()
            .map(|t| t.skip.get(name))
            .flatten()
            .is_none()
            .then(|| {
                quote!(
                    #arg_name = ::juniper::tracing::field::debug(
                        args.get::<#ty>(#arg_getter).unwrap_or_else(|| {
                            ::juniper::FromInputValue::<#scalar>::from_implicit_null()
                        })
                    )
                )
            })
    });

    let args: Vec<_> = if let Some(tracing) = field.tracing_attr() {
        let additional_fields = tracing.fields.iter().map(|f| {
            let name = &f.left;
            let right = &f.right;
            quote!(#name = ::juniper::tracing::field::debug(#right))
        });

        args.chain(additional_fields).collect()
    } else {
        args.collect()
    };

    let level = field
        .tracing_attr()
        .map(|t| t.level.as_ref())
        .flatten()
        .map(|l| match l.value().as_str() {
            "trace" => quote!(TRACE),
            "debug" => quote!(DEBUG),
            "info" => quote!(INFO),
            "warn" => quote!(WARN),
            "error" => quote!(ERROR),
            l => abort!(syn::Error::new(
                l.span(),
                format!(
                    "Unsupported tracing level: {}, \
                     supported values: trace, debug, info, warn, error",
                    l,
                ),
            )),
        })
        .unwrap_or_else(|| quote!(INFO));

    let target = field
        .tracing_attr()
        .map(|t| t.target.as_ref())
        .flatten()
        .map_or_else(|| quote!(), |t| quote!(target: #t,));

    quote!(
        let _tracing_span = ::juniper::tracing::span!(
            #target ::juniper::tracing::Level::#level, #span_name, #(#args, )*
        );
    )
}

// Returns code to start tracing of async future
pub fn async_tokens(ty: &impl TracedType, field: &impl TracedField) -> TokenStream {
    if !is_traced(ty, field) {
        return quote!();
    }
    quote!(.__instrument(_tracing_span))
}

// Returns code to start tracing of sync block
pub fn sync_tokens(ty: &impl TracedType, field: &impl TracedField) -> TokenStream {
    if !is_traced(ty, field) {
        return quote!();
    }
    quote!(let _tracing_guard = _tracing_span.enter();)
}

mod graphql_object {
    use crate::util::{
        GraphQLTypeDefinitionField, GraphQLTypeDefinitionFieldArg, GraphQLTypeDefiniton,
    };

    use super::{Attr, TracedArgument, TracedField, TracedType, TracingRule};

    impl TracedType for GraphQLTypeDefiniton {
        fn tracing_rule(&self) -> Option<TracingRule> {
            self.tracing_rule
        }

        fn name(&self) -> &str {
            self.name.as_str()
        }

        fn scalar(&self) -> Option<syn::Type> {
            self.scalar.clone()
        }
    }

    impl TracedField for GraphQLTypeDefinitionField {
        type Arg = GraphQLTypeDefinitionFieldArg;

        fn tracing_attr(&self) -> Option<&Attr> {
            self.tracing.as_ref()
        }

        fn name(&self) -> &str {
            self.name.as_str()
        }

        fn args(&self) -> Vec<&Self::Arg> {
            self.args.iter().collect()
        }

        fn is_async(&self) -> bool {
            self.is_async
        }
    }

    impl TracedArgument for GraphQLTypeDefinitionFieldArg {
        fn ty(&self) -> &syn::Type {
            &self._type
        }

        fn name(&self) -> &str {
            self.name.as_str()
        }
    }
}
