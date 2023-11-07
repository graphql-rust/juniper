//! Common functions, definitions and extensions for parsing, normalizing and modifying Rust syntax,
//! used by this crate.

pub(crate) mod attr;
pub(crate) mod downcaster;

use std::{any::TypeId, iter, mem};

use proc_macro2::Span;
use quote::quote;
use syn::{
    ext::IdentExt as _,
    parse::{Parse, ParseBuffer},
    parse_quote,
    punctuated::Punctuated,
    token::{self, Token},
    visit_mut::VisitMut,
};

/// Extension of [`ParseBuffer`] providing common function widely used by this crate for parsing.
pub(crate) trait ParseBufferExt {
    /// Tries to parse `T` as the next token.
    ///
    /// Doesn't move [`ParseStream`]'s cursor if there is no `T`.
    fn try_parse<T: Default + Parse + Token>(&self) -> syn::Result<Option<T>>;

    /// Checks whether next token is `T`.
    ///
    /// Doesn't move [`ParseStream`]'s cursor.
    #[must_use]
    fn is_next<T: Default + Token>(&self) -> bool;

    /// Parses next token as [`syn::Ident`] _allowing_ Rust keywords, while default [`Parse`]
    /// implementation for [`syn::Ident`] disallows keywords.
    ///
    /// Always moves [`ParseStream`]'s cursor.
    fn parse_any_ident(&self) -> syn::Result<syn::Ident>;

    /// Checks whether next token is a wrapper `W` and if yes, then parses the wrapped tokens as `T`
    /// [`Punctuated`] with `P`. Otherwise, parses just `T`.
    ///
    /// Always moves [`ParseStream`]'s cursor.
    fn parse_maybe_wrapped_and_punctuated<T, W, P>(&self) -> syn::Result<Punctuated<T, P>>
    where
        T: Parse,
        W: Default + Token + 'static,
        P: Default + Parse + Token;
}

impl<'a> ParseBufferExt for ParseBuffer<'a> {
    fn try_parse<T: Default + Parse + Token>(&self) -> syn::Result<Option<T>> {
        Ok(if self.is_next::<T>() {
            Some(self.parse()?)
        } else {
            None
        })
    }

    fn is_next<T: Default + Token>(&self) -> bool {
        self.lookahead1().peek(|_| T::default())
    }

    fn parse_any_ident(&self) -> syn::Result<syn::Ident> {
        self.call(syn::Ident::parse_any)
    }

    fn parse_maybe_wrapped_and_punctuated<T, W, P>(&self) -> syn::Result<Punctuated<T, P>>
    where
        T: Parse,
        W: Default + Token + 'static,
        P: Default + Parse + Token,
    {
        Ok(if self.is_next::<W>() {
            let inner;
            if TypeId::of::<W>() == TypeId::of::<token::Bracket>() {
                let _ = syn::bracketed!(inner in self);
            } else if TypeId::of::<W>() == TypeId::of::<token::Brace>() {
                let _ = syn::braced!(inner in self);
            } else if TypeId::of::<W>() == TypeId::of::<token::Paren>() {
                let _ = syn::parenthesized!(inner in self);
            } else {
                unimplemented!(
                    "ParseBufferExt::parse_maybe_wrapped_and_punctuated supports only brackets, \
                     braces and parentheses as wrappers.",
                );
            }
            Punctuated::parse_terminated(&inner)?
        } else {
            Punctuated::from_iter(iter::once(self.parse::<T>()?))
        })
    }
}

/// Extension of [`syn::Type`] providing common function widely used by this crate for parsing.
pub(crate) trait TypeExt {
    /// Retrieves the innermost non-parenthesized [`syn::Type`] from the given
    /// one (unwraps nested [`syn::TypeParen`]s asap).
    #[must_use]
    fn unparenthesized(&self) -> &Self;

    /// Retrieves the inner [`syn::Type`] from the given reference type, or just
    /// returns "as is" if the type is not a reference.
    ///
    /// Also, makes the type [`TypeExt::unparenthesized`], if possible.
    #[must_use]
    fn unreferenced(&self) -> &Self;

    /// Iterates mutably over all the lifetime parameters of this [`syn::Type`]
    /// with the given `func`tion.
    fn lifetimes_iter_mut<F: FnMut(&mut syn::Lifetime)>(&mut self, func: &mut F);

    /// Anonymizes all the lifetime parameters of this [`syn::Type`] (except
    /// the `'static` ones), making it suitable for using in contexts with
    /// inferring.
    fn lifetimes_anonymized(&mut self);

    /// Returns the topmost [`syn::Ident`] of this [`syn::TypePath`], if any.
    #[must_use]
    fn topmost_ident(&self) -> Option<&syn::Ident>;
}

impl TypeExt for syn::Type {
    fn unparenthesized(&self) -> &Self {
        match self {
            Self::Paren(ty) => ty.elem.unparenthesized(),
            Self::Group(ty) => ty.elem.unparenthesized(),
            ty => ty,
        }
    }

    fn unreferenced(&self) -> &Self {
        match self.unparenthesized() {
            Self::Reference(ref_ty) => &ref_ty.elem,
            ty => ty,
        }
    }

    fn lifetimes_iter_mut<F: FnMut(&mut syn::Lifetime)>(&mut self, func: &mut F) {
        use syn::{GenericArgument as GA, Type as T};

        fn iter_path<F: FnMut(&mut syn::Lifetime)>(path: &mut syn::Path, func: &mut F) {
            for seg in path.segments.iter_mut() {
                match &mut seg.arguments {
                    syn::PathArguments::AngleBracketed(angle) => {
                        for arg in angle.args.iter_mut() {
                            match arg {
                                GA::Lifetime(lt) => func(lt),
                                GA::Type(ty) => ty.lifetimes_iter_mut(func),
                                GA::AssocType(a) => a.ty.lifetimes_iter_mut(func),
                                GA::Constraint(_) | GA::AssocConst(_) | GA::Const(_) => {}
                                // Following the `syn` idiom for exhaustive matching on `Type`:
                                // https://docs.rs/syn/2.0.38/src/syn/ty.rs.html#64-79
                                // TODO: #[cfg_attr(test, deny(non_exhaustive_omitted_patterns))]
                                //       https://github.com/rust-lang/rust/issues/89554
                                _ => unimplemented!(),
                            }
                        }
                    }
                    syn::PathArguments::Parenthesized(args) => {
                        for ty in args.inputs.iter_mut() {
                            ty.lifetimes_iter_mut(func)
                        }
                        if let syn::ReturnType::Type(_, ty) = &mut args.output {
                            (*ty).lifetimes_iter_mut(func)
                        }
                    }
                    syn::PathArguments::None => {}
                }
            }
        }

        match self {
            T::Array(syn::TypeArray { elem, .. })
            | T::Group(syn::TypeGroup { elem, .. })
            | T::Paren(syn::TypeParen { elem, .. })
            | T::Ptr(syn::TypePtr { elem, .. })
            | T::Slice(syn::TypeSlice { elem, .. }) => (*elem).lifetimes_iter_mut(func),

            T::Tuple(syn::TypeTuple { elems, .. }) => {
                for ty in elems.iter_mut() {
                    ty.lifetimes_iter_mut(func)
                }
            }

            T::ImplTrait(syn::TypeImplTrait { bounds, .. })
            | T::TraitObject(syn::TypeTraitObject { bounds, .. }) => {
                for bound in bounds.iter_mut() {
                    match bound {
                        syn::TypeParamBound::Lifetime(lt) => func(lt),
                        syn::TypeParamBound::Trait(bound) => {
                            if bound.lifetimes.is_some() {
                                todo!("Iterating over HRTB lifetimes in trait is not yet supported")
                            }
                            iter_path(&mut bound.path, func)
                        }
                        syn::TypeParamBound::Verbatim(_) => {}
                        // Following the `syn` idiom for exhaustive matching on `Type`:
                        // https://docs.rs/syn/2.0.38/src/syn/ty.rs.html#64-79
                        // TODO: #[cfg_attr(test, deny(non_exhaustive_omitted_patterns))]
                        //       https://github.com/rust-lang/rust/issues/89554
                        _ => unimplemented!(),
                    }
                }
            }

            T::Reference(ref_ty) => {
                if let Some(lt) = ref_ty.lifetime.as_mut() {
                    func(lt)
                }
                (*ref_ty.elem).lifetimes_iter_mut(func)
            }

            T::Path(ty) => iter_path(&mut ty.path, func),

            // These types unlikely will be used as GraphQL types.
            T::BareFn(_) | T::Infer(_) | T::Macro(_) | T::Never(_) | T::Verbatim(_) => {}

            // Following the `syn` idiom for exhaustive matching on `Type`:
            // https://docs.rs/syn/2.0.38/src/syn/ty.rs.html#64-79
            // TODO: #[cfg_attr(test, deny(non_exhaustive_omitted_patterns))]
            //       https://github.com/rust-lang/rust/issues/89554
            _ => unimplemented!(),
        }
    }

    fn lifetimes_anonymized(&mut self) {
        self.lifetimes_iter_mut(&mut |lt| {
            if lt.ident != "_" && lt.ident != "static" {
                lt.ident = syn::Ident::new("_", Span::call_site());
            }
        });
    }

    fn topmost_ident(&self) -> Option<&syn::Ident> {
        match self.unparenthesized() {
            syn::Type::Path(p) => Some(&p.path),
            syn::Type::Reference(r) => match (*r.elem).unparenthesized() {
                syn::Type::Path(p) => Some(&p.path),
                syn::Type::TraitObject(o) => match o.bounds.iter().next().unwrap() {
                    syn::TypeParamBound::Trait(b) => Some(&b.path),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }?
        .segments
        .last()
        .map(|s| &s.ident)
    }
}

/// Extension of [`syn::Generics`] providing common function widely used by this crate for parsing.
pub(crate) trait GenericsExt {
    /// Removes all default types out of type parameters and const parameters in these
    /// [`syn::Generics`].
    fn remove_defaults(&mut self);

    /// Moves all trait and lifetime bounds of these [`syn::Generics`] to its [`syn::WhereClause`].
    fn move_bounds_to_where_clause(&mut self);

    /// Replaces generic parameters in the given [`syn::Type`] with default
    /// ones, provided by these [`syn::Generics`].
    fn replace_type_with_defaults(&self, ty: &mut syn::Type);

    /// Replaces generic parameters in the given [`syn::TypePath`] with default
    /// ones, provided by these [`syn::Generics`].
    fn replace_type_path_with_defaults(&self, ty: &mut syn::TypePath);
}

impl GenericsExt for syn::Generics {
    fn remove_defaults(&mut self) {
        use syn::GenericParam as P;

        for p in &mut self.params {
            match p {
                P::Type(p) => {
                    p.eq_token = None;
                    p.default = None;
                }
                P::Lifetime(_) => {}
                P::Const(p) => {
                    p.eq_token = None;
                    p.default = None;
                }
            }
        }
    }

    fn move_bounds_to_where_clause(&mut self) {
        use syn::GenericParam as P;

        let _ = self.make_where_clause();
        let where_clause = self.where_clause.as_mut().unwrap();

        for p in &mut self.params {
            match p {
                P::Type(p) => {
                    if p.colon_token.is_some() {
                        p.colon_token = None;
                        let bounds = mem::take(&mut p.bounds);
                        let ty = &p.ident;
                        where_clause.predicates.push(parse_quote! { #ty: #bounds });
                    }
                }
                P::Lifetime(p) => {
                    if p.colon_token.is_some() {
                        p.colon_token = None;
                        let bounds = mem::take(&mut p.bounds);
                        let lt = &p.lifetime;
                        where_clause.predicates.push(parse_quote! { #lt: #bounds });
                    }
                }
                P::Const(_) => {}
            }
        }
    }

    fn replace_type_with_defaults(&self, ty: &mut syn::Type) {
        ReplaceWithDefaults(self).visit_type_mut(ty)
    }

    fn replace_type_path_with_defaults(&self, ty: &mut syn::TypePath) {
        ReplaceWithDefaults(self).visit_type_path_mut(ty)
    }
}

/// Replaces [`Generics`] with default values:
/// - `'static` for [`Lifetime`]s;
/// - `::juniper::DefaultScalarValue` for [`Type`]s.
///
/// [`Generics`]: syn::Generics
/// [`Lifetime`]: syn::Lifetime
/// [`Type`]: syn::Type
struct ReplaceWithDefaults<'a>(&'a syn::Generics);

impl<'a> VisitMut for ReplaceWithDefaults<'a> {
    fn visit_generic_argument_mut(&mut self, arg: &mut syn::GenericArgument) {
        match arg {
            syn::GenericArgument::Lifetime(lf) => {
                *lf = parse_quote! { 'static };
            }
            syn::GenericArgument::Type(ty) => {
                let is_generic = self
                    .0
                    .params
                    .iter()
                    .filter_map(|par| match par {
                        syn::GenericParam::Type(ty) => Some(&ty.ident),
                        _ => None,
                    })
                    .any(|par| {
                        let par = quote! { #par }.to_string();
                        let ty = quote! { #ty }.to_string();
                        par == ty
                    });

                if is_generic {
                    // Replace with `DefaultScalarValue` instead of `()`
                    // because generic parameter may be scalar.
                    *ty = parse_quote!(::juniper::DefaultScalarValue);
                }
            }
            _ => {}
        }
    }
}
