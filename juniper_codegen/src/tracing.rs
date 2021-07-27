use std::{collections::HashMap, mem, str::FromStr};

use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned as _,
    token,
};

pub const ATTR_NAME: &'static str = "instrument";

#[derive(Debug, Default)]
pub struct Attr {
    /// Optional span rename, if `None` method name should be used instead.
    name: Option<syn::LitStr>,

    /// Overwritten `level` of span generated, if `None` `Level::INFO` should be used.
    level: Option<syn::LitStr>,

    /// Overwritten `target` of span.
    target: Option<syn::LitStr>,

    /// Skipped arguments on `fn` resolvers.
    skip: HashMap<String, syn::Ident>,

    /// Custom fields.
    fields: Vec<syn::ExprAssign>,
}

impl Attr {
    /// Parses [`TracingAttr`] from `method`s attributes and removes itself from
    /// `method.attrs` if present.
    pub fn from_trait_method(method: &mut syn::TraitItemMethod) -> Option<Self> {
        let attr = method
            .attrs
            .iter()
            .find(|attr| attr.path.is_ident(&ATTR_NAME))
            .map(|attr| attr.parse_args())
            .transpose();

        method.attrs = mem::take(&mut method.attrs)
            .into_iter()
            .filter(|attr| !attr.path.is_ident(&ATTR_NAME))
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
pub enum Rule {
    /// Trace all fields.
    All,

    /// Trace all fields that resolved using `async fn`s.
    Async,

    /// Trace all fields that can be synchronously resolved.
    Sync,

    /// Trace only fields that marked with `#[tracing(complex)]`.
    Complex,

    /// Skip tracing of all fields.
    SkipAll,
}

impl Default for Rule {
    fn default() -> Self {
        Self::All
    }
}

impl FromStr for Rule {
    type Err = ();

    fn from_str(rule: &str) -> Result<Self, Self::Err> {
        match rule {
            "async" => Ok(Self::Async),
            "sync" => Ok(Self::Sync),
            "skip_all" => Ok(Self::SkipAll),
            "only" => Ok(Self::Complex),
            _ => Err(()),
        }
    }
}

/// Marker on field which used together with [`Rule`] to decide whether this
/// field should be traced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldBehaviour {
    /// Default tracing behaviour.
    ///
    /// It means that field **should** be traced if nothing else restricting it.
    Default,

    /// Used together with `tracing(only)` argument to mark that field should be traced.
    Only,

    /// Used to mark that field shouldn't be traced at all.
    Ignore,
}

impl FieldBehaviour {
    pub fn from_ident(ident: &syn::Ident) -> syn::Result<Self> {
        match ident.to_string().as_str() {
            "only" => Ok(Self::Only),
            "ignore" | "skip" => Ok(Self::Ignore),
            _ => Err(syn::Error::new(
                ident.span(),
                format!(
                    "Unknown tracing behaviour: got {}, supported values: only, ignore, skip",
                    ident,
                ),
            )),
        }
    }
}

/// Generalisation of type that can be traced.
pub trait TracedType {
    /// Optional [`TracingRule`] read from attributes `#[graphql_object(trace = "...")]`
    /// on impl block, `#[graphql(trace = "...")]` on derived GraphQLObject or
    /// `#[graphql_interface(trace = "...")]` on trait definition.
    fn tracing_rule(&self) -> Rule;

    /// Name of this type.
    fn name(&self) -> &str;

    /// Scalar used by this GraphQL object.
    fn scalar(&self) -> Option<syn::Type>;
}

/// Trait that marks type that this is field that can be traced.
pub trait TracedField {
    /// Type of argument used by this field.
    type Arg: TracedArgument;

    /// Returns parsed `#[tracing]` attribute.
    fn instrument(&self) -> Option<&Attr>;

    /// Returns [`FieldBehaviour`] parsed from `#[graphql(tracing = ...)]`
    fn tracing_behaviour(&self) -> FieldBehaviour;

    /// Whether this field relies on async resolver.
    fn is_async(&self) -> bool;

    /// Name of this field.
    fn name(&self) -> &str;

    /// Arguments if resolver is `fn`.
    fn args(&self) -> Vec<&Self::Arg>;
}

pub trait TracedArgument {
    fn ty(&self) -> &syn::Type;
    fn name(&self) -> &str;
    fn raw_name(&self) -> &syn::Ident;
}

fn is_traced(ty: &impl TracedType, field: &impl TracedField) -> bool {
    let rule = ty.tracing_rule();

    match rule {
        Rule::All => field.tracing_behaviour() != FieldBehaviour::Ignore,
        Rule::Sync if !field.is_async() => field.tracing_behaviour() != FieldBehaviour::Ignore,
        Rule::Async if field.is_async() => field.tracing_behaviour() != FieldBehaviour::Ignore,
        Rule::Complex => field.tracing_behaviour() == FieldBehaviour::Only,
        _ => false,
    }
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
        let raw_name = arg.raw_name();
        let arg_name = syn::LitStr::new(name, arg.ty().span());

        field
            .instrument()
            .map(|t| t.skip.get(&raw_name.to_string()))
            .flatten()
            .is_none()
            .then(|| {
                quote!(
                    #arg_name = ::juniper::tracing::field::debug(&#raw_name)
                )
            })
    });

    let args: Vec<_> = if let Some(tracing) = field.instrument() {
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
        .instrument()
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
        .instrument()
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
    quote! (
        let f = <_ as ::juniper::tracing_futures::Instrument>::instrument(f, _tracing_span);
    )
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
        GraphQLTypeDefinition, GraphQLTypeDefinitionField, GraphQLTypeDefinitionFieldArg,
    };

    use super::{Attr, Rule, TracedArgument, TracedField, TracedType};
    use crate::tracing::FieldBehaviour;

    impl TracedType for GraphQLTypeDefinition {
        fn tracing_rule(&self) -> Rule {
            self.tracing_rule.unwrap_or(Rule::All)
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

        fn instrument(&self) -> Option<&Attr> {
            self.instrument_attr.as_ref()
        }

        fn tracing_behaviour(&self) -> FieldBehaviour {
            self.tracing_behaviour.unwrap_or(FieldBehaviour::Default)
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

        fn raw_name(&self) -> &syn::Ident {
            &self.raw_name
        }
    }
}
