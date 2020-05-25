use proc_macro2::{Span, TokenStream};
use proc_macro_error::ResultExt as _;
use quote::{quote, ToTokens as _};
use syn::{self, ext::IdentExt, parse_quote, spanned::Spanned as _, Data, Fields};

use crate::{
    result::GraphQLScope,
    util::{span_container::SpanContainer, Mode},
};

use super::{UnionMeta, UnionVariantMeta};

const SCOPE: GraphQLScope = GraphQLScope::DeriveUnion;

/// Expands `#[derive(GraphQLUnion)]` macro into generated code.
pub fn expand(input: TokenStream, mode: Mode) -> syn::Result<TokenStream> {
    let ast = syn::parse2::<syn::DeriveInput>(input).unwrap_or_abort();

    match &ast.data {
        Data::Enum(_) => expand_enum(ast, mode),
        Data::Struct(_) => unimplemented!(), // TODO
        _ => Err(SCOPE.custom_error(ast.span(), "can only be applied to enums and structs")),
    }
    .map(UnionDefinition::into_tokens)
}

fn expand_enum(ast: syn::DeriveInput, mode: Mode) -> syn::Result<UnionDefinition> {
    let meta = UnionMeta::from_attrs(&ast.attrs)?;

    let enum_span = ast.span();
    let enum_ident = ast.ident;

    let name = meta
        .name
        .clone()
        .map(SpanContainer::into_inner)
        .unwrap_or_else(|| enum_ident.unraw().to_string()); // TODO: PascalCase
    if matches!(mode, Mode::Public) && name.starts_with("__") {
        SCOPE.no_double_underscore(
            meta.name
                .as_ref()
                .map(SpanContainer::span_ident)
                .unwrap_or_else(|| enum_ident.span()),
        );
    }

    let mut variants: Vec<_> = match ast.data {
        Data::Enum(data) => data.variants,
        _ => unreachable!(),
    }
    .into_iter()
    .filter_map(|var| graphql_union_variant_from_enum_variant(var, &enum_ident, &meta, mode))
    .collect();
    if !meta.custom_resolvers.is_empty() {
        let crate_path = mode.crate_path();
        // TODO: refactor into separate function
        for (ty, rslvr) in meta.custom_resolvers {
            let span = rslvr.span_joined();

            let resolver_fn = rslvr.into_inner();
            let resolver_code = parse_quote! {
                #resolver_fn(self, #crate_path::FromContext::from(context))
            };
            // Doing this may be quite an expensive, because resolving may contain some heavy
            // computation, so we're preforming it twice. Unfortunately, we have no other options
            // here, until the `juniper::GraphQLType` itself will allow to do it in some cleverer
            // way.
            let resolver_check = parse_quote! {
                ({ #resolver_code } as ::std::option::Option<&#ty>).is_some()
            };

            if let Some(var) = variants.iter_mut().find(|v| v.ty == ty) {
                var.resolver_code = resolver_code;
                var.resolver_check = resolver_check;
                var.span = span;
            } else {
                variants.push(UnionVariantDefinition {
                    ty,
                    resolver_code,
                    resolver_check,
                    enum_path: None,
                    span,
                })
            }
        }
    }
    if variants.is_empty() {
        SCOPE.not_empty(enum_span);
    }

    // NOTICE: This is not an optimal implementation, as it's possible to bypass this check by using
    // a full qualified path instead (`crate::Test` vs `Test`). Since this requirement is mandatory,
    // the `std::convert::Into<T>` implementation is used to enforce this requirement. However, due
    // to the bad error message this implementation should stay and provide guidance.
    let all_variants_different = {
        let mut types: Vec<_> = variants.iter().map(|var| &var.ty).collect();
        types.dedup();
        types.len() == variants.len()
    };
    if !all_variants_different {
        SCOPE.custom(enum_ident.span(), "each variant must have a different type");
    }

    proc_macro_error::abort_if_dirty();

    Ok(UnionDefinition {
        name,
        ty: syn::parse_str(&enum_ident.to_string()).unwrap_or_abort(),
        description: meta.description.map(SpanContainer::into_inner),
        context: meta.context.map(SpanContainer::into_inner),
        scalar: meta.scalar.map(SpanContainer::into_inner),
        generics: ast.generics,
        variants,
        span: enum_span,
        mode,
    })
}

fn graphql_union_variant_from_enum_variant(
    var: syn::Variant,
    enum_ident: &syn::Ident,
    enum_meta: &UnionMeta,
    mode: Mode,
) -> Option<UnionVariantDefinition> {
    let meta = UnionVariantMeta::from_attrs(&var.attrs)
        .map_err(|e| proc_macro_error::emit_error!(e))
        .ok()?;
    if meta.ignore.is_some() {
        return None;
    }

    let var_span = var.span();
    let var_ident = var.ident;

    let ty = match var.fields {
        Fields::Unnamed(fields) => {
            let mut iter = fields.unnamed.iter();
            let first = iter.next().unwrap();
            if iter.next().is_none() {
                Ok(first.ty.clone())
            } else {
                Err(fields.span())
            }
        }
        _ => Err(var_ident.span()),
    }
    .map_err(|span| {
        SCOPE.custom(
            span,
            "only unnamed variants with a single field are allowed, e.g. Some(T)",
        )
    })
    .ok()?;

    let enum_path = quote! { #enum_ident::#var_ident };

    let resolver_code = if let Some(rslvr) = meta.custom_resolver {
        if let Some(other) = enum_meta.custom_resolvers.get(&ty) {
            SCOPE.custom(
                rslvr.span_ident(),
                format!(
                    "variant `{}` already has custom resolver `{}` declared on the enum",
                    ty.to_token_stream(),
                    other.to_token_stream(),
                ),
            );
        }

        let crate_path = mode.crate_path();
        let resolver_fn = rslvr.into_inner();

        parse_quote! {
            #resolver_fn(self, #crate_path::FromContext::from(context))
        }
    } else {
        parse_quote! {
            match self { #enum_ident::#var_ident(ref v) => Some(v), _ => None, }
        }
    };

    let resolver_check = parse_quote! {
        matches!(self, #enum_path(_))
    };

    Some(UnionVariantDefinition {
        ty,
        resolver_code,
        resolver_check,
        enum_path: Some(enum_path),
        span: var_span,
    })
}

struct UnionVariantDefinition {
    pub ty: syn::Type,
    pub resolver_code: syn::Expr,
    pub resolver_check: syn::Expr,
    pub enum_path: Option<TokenStream>,
    pub span: Span,
}

struct UnionDefinition {
    pub name: String,
    pub ty: syn::Type,
    pub description: Option<String>,
    pub context: Option<syn::Type>,
    pub scalar: Option<syn::Type>,
    pub generics: syn::Generics,
    pub variants: Vec<UnionVariantDefinition>,
    pub span: Span,
    pub mode: Mode,
}

impl UnionDefinition {
    pub fn into_tokens(self) -> TokenStream {
        let crate_path = self.mode.crate_path();

        let name = &self.name;
        let ty = &self.ty;

        let context = self
            .context
            .as_ref()
            .map(|ctx| quote! { #ctx })
            .unwrap_or_else(|| quote! { () });

        let scalar = self
            .scalar
            .as_ref()
            .map(|scl| quote! { #scl })
            .unwrap_or_else(|| quote! { __S });
        let default_scalar = self
            .scalar
            .as_ref()
            .map(|scl| quote! { #scl })
            .unwrap_or_else(|| quote! { #crate_path::DefaultScalarValue });

        let description = self
            .description
            .as_ref()
            .map(|desc| quote! { .description(#desc) });

        let var_types: Vec<_> = self.variants.iter().map(|var| &var.ty).collect();

        let match_names = self.variants.iter().map(|var| {
            let var_ty = &var.ty;
            let var_check = &var.resolver_check;
            quote! {
                if #var_check {
                    return <#var_ty as #crate_path::GraphQLType<#scalar>>::name(&())
                        .unwrap().to_string();
                }
            }
        });

        let match_resolves: Vec<_> = self.variants.iter().map(|var| &var.resolver_code).collect();
        let resolve_into_type = self.variants.iter().zip(match_resolves.iter()).map(|(var, expr)| {
            let var_ty = &var.ty;

            let get_name = quote! { (<#var_ty as #crate_path::GraphQLType<#scalar>>::name(&())) };
            quote! {
                if type_name == #get_name.unwrap() {
                    return #crate_path::IntoResolvable::into(
                        { #expr },
                        executor.context()
                    )
                    .and_then(|res| match res {
                        Some((ctx, r)) => executor.replaced_context(ctx).resolve_with_ctx(&(), &r),
                        None => Ok(#crate_path::Value::null()),
                    });
                }
            }
        });
        let resolve_into_type_async =
            self.variants
                .iter()
                .zip(match_resolves.iter())
                .map(|(var, expr)| {
                    let var_ty = &var.ty;

                    let get_name =
                        quote! { (<#var_ty as #crate_path::GraphQLType<#scalar>>::name(&())) };
                    quote! {
                        if type_name == #get_name.unwrap() {
                            let res = #crate_path::IntoResolvable::into(
                                { #expr },
                                executor.context()
                            );
                            return #crate_path::futures::future::FutureExt::boxed(async move {
                                match res? {
                                    Some((ctx, r)) => {
                                        let subexec = executor.replaced_context(ctx);
                                        subexec.resolve_with_ctx_async(&(), &r).await
                                    },
                                    None => Ok(#crate_path::Value::null()),
                                }
                            });
                        }
                    }
                });

        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();
        let mut ext_generics = self.generics.clone();
        if self.scalar.is_none() {
            ext_generics.params.push(parse_quote! { #scalar });
            ext_generics
                .where_clause
                .get_or_insert_with(|| parse_quote! { where })
                .predicates
                .push(parse_quote! { #scalar: #crate_path::ScalarValue });
        }
        let (ext_impl_generics, _, where_clause) = ext_generics.split_for_impl();

        let mut where_async = where_clause
            .cloned()
            .unwrap_or_else(|| parse_quote! { where });
        where_async
            .predicates
            .push(parse_quote! { Self: Send + Sync });
        if self.scalar.is_none() {
            where_async
                .predicates
                .push(parse_quote! { #scalar: Send + Sync });
        }

        let type_impl = quote! {
            #[automatically_derived]
            impl#ext_impl_generics #crate_path::GraphQLType<#scalar> for #ty#ty_generics
                #where_clause
            {
                type Context = #context;
                type TypeInfo = ();

                fn name(_ : &Self::TypeInfo) -> Option<&str> {
                    Some(#name)
                }

                fn meta<'r>(
                    info: &Self::TypeInfo,
                    registry: &mut #crate_path::Registry<'r, #scalar>
                ) -> #crate_path::meta::MetaType<'r, #scalar>
                where #scalar: 'r,
                {
                    let types = &[
                        #( registry.get_type::<&#var_types>(&(())), )*
                    ];
                    registry.build_union_type::<#ty#ty_generics>(info, types)
                    #description
                    .into_meta()
                }

                fn concrete_type_name(
                    &self,
                    context: &Self::Context,
                    _: &Self::TypeInfo,
                ) -> String {
                    #( #match_names )*
                    panic!(
                        "GraphQL union {} cannot be resolved into any of its variants in its \
                         current state",
                        #name,
                    );
                }

                fn resolve_into_type(
                    &self,
                    _: &Self::TypeInfo,
                    type_name: &str,
                    _: Option<&[#crate_path::Selection<#scalar>]>,
                    executor: &#crate_path::Executor<Self::Context, #scalar>,
                ) -> #crate_path::ExecutionResult<#scalar> {
                    let context = executor.context();
                    #( #resolve_into_type )*
                    panic!(
                        "Concrete type {} is not handled by instance resolvers on GraphQL union {}",
                        type_name, #name,
                    );
                }
            }
        };

        let async_type_impl = quote! {
            #[automatically_derived]
            impl#ext_impl_generics #crate_path::GraphQLTypeAsync<#scalar> for #ty#ty_generics
                #where_async
            {
                fn resolve_into_type_async<'b>(
                    &'b self,
                    _: &'b Self::TypeInfo,
                    type_name: &str,
                    _: Option<&'b [#crate_path::Selection<'b, #scalar>]>,
                    executor: &'b #crate_path::Executor<'b, 'b, Self::Context, #scalar>
                ) -> #crate_path::BoxFuture<'b, #crate_path::ExecutionResult<#scalar>> {
                    let context = executor.context();
                    #( #resolve_into_type_async )*
                    panic!(
                        "Concrete type {} is not handled by instance resolvers on GraphQL union {}",
                        type_name, #name,
                    );
                }
            }
        };

        let output_type_impl = quote! {
            #[automatically_derived]
            impl#ext_impl_generics #crate_path::marker::IsOutputType<#scalar> for #ty#ty_generics
                #where_clause
            {
                fn mark() {
                    #( <#var_types as #crate_path::marker::GraphQLObjectType<#scalar>>::mark(); )*
                }
            }
        };

        let union_impl = quote! {
            #[automatically_derived]
            impl#impl_generics #crate_path::marker::GraphQLUnion for #ty#ty_generics {
                fn mark() {
                    #( <#var_types as #crate_path::marker::GraphQLObjectType<
                        #default_scalar,
                    >>::mark(); )*
                }
            }
        };

        quote! {
            #union_impl
            #output_type_impl
            #type_impl
            #async_type_impl
        }
    }
}
