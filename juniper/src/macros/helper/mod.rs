//! Helper traits and definitions for macros.

pub mod subscription;

use std::{fmt, mem, rc::Rc, sync::Arc};

use futures::future::{self, BoxFuture};

use crate::{
    Arguments, DefaultScalarValue, DynGraphQLValue, DynGraphQLValueAsync, ExecutionResult,
    Executor, FieldError, Nullable, ScalarValue,
};

/// Conversion of a [`GraphQLValue`] to its [trait object][1].
///
/// [`GraphQLValue`]: crate::GraphQLValue
/// [1]: https://doc.rust-lang.org/reference/types/trait-object.html
pub trait AsDynGraphQLValue<S: ScalarValue = DefaultScalarValue> {
    /// Context type of this [`GraphQLValue`].
    ///
    /// [`GraphQLValue`]: crate::GraphQLValue
    type Context;

    /// Schema information type of this [`GraphQLValue`].
    ///
    /// [`GraphQLValue`]: crate::GraphQLValue
    type TypeInfo;

    /// Converts this value to a [`DynGraphQLValue`] [trait object][1].
    ///
    /// [1]: https://doc.rust-lang.org/reference/types/trait-object.html
    fn as_dyn_graphql_value(&self) -> &DynGraphQLValue<S, Self::Context, Self::TypeInfo>;

    /// Converts this value to a [`DynGraphQLValueAsync`] [trait object][1].
    ///
    /// [1]: https://doc.rust-lang.org/reference/types/trait-object.html
    fn as_dyn_graphql_value_async(&self)
        -> &DynGraphQLValueAsync<S, Self::Context, Self::TypeInfo>;
}

crate::sa::assert_obj_safe!(AsDynGraphQLValue<Context = (), TypeInfo = ()>);

/// This trait is used by [`graphql_scalar!`] macro to retrieve [`Error`] type
/// from a [`Result`].
///
/// [`Error`]: Result::Error
/// [`graphql_scalar!`]: macro@crate::graphql_scalar
pub trait ExtractError {
    /// Extracted [`Error`] type of this [`Result`].
    ///
    /// [`Error`]: Result::Error
    type Error;
}

impl<T, E> ExtractError for Result<T, E> {
    type Error = E;
}

/// Wraps `msg` with [`Display`] implementation into opaque [`Send`] [`Future`]
/// which immediately resolves into [`FieldError`].
pub fn err_fut<'ok, D, Ok, S>(msg: D) -> BoxFuture<'ok, Result<Ok, FieldError<S>>>
where
    D: fmt::Display,
    Ok: Send + 'ok,
    S: Send + 'static,
{
    Box::pin(future::err(FieldError::from(msg)))
}

/// Generates a [`FieldError`] for the given Rust type expecting to have
/// [`GraphQLType::name`].
///
/// [`GraphQLType::name`]: crate::GraphQLType::name
pub fn err_unnamed_type<S>(name: &str) -> FieldError<S> {
    FieldError::from(format!(
        "Expected `{}` type to implement `GraphQLType::name`",
        name,
    ))
}

/// Returns a [`future::err`] wrapping the [`err_unnamed_type`].
pub fn err_unnamed_type_fut<'ok, Ok, S>(name: &str) -> BoxFuture<'ok, Result<Ok, FieldError<S>>>
where
    Ok: Send + 'ok,
    S: Send + 'static,
{
    Box::pin(future::err(err_unnamed_type(name)))
}

pub type Type = &'static str;
pub type Types = &'static [Type];
pub type Name = &'static str;
pub type FieldName = u128;
pub type WrappedValue = u128;

/// Non-cryptographic hash with good dispersion to use [`str`](prim@str) in
/// const generics. See [spec] for more info.
///
/// [spec]: https://datatracker.ietf.org/doc/html/draft-eastlake-fnv-17.html
pub const fn fnv1a128(str: Name) -> u128 {
    const FNV_OFFSET_BASIS: u128 = 0x6c62272e07bb014262b821756295c58d;
    const FNV_PRIME: u128 = 0x0000000001000000000000000000013b;

    let bytes = str.as_bytes();
    let mut hash = FNV_OFFSET_BASIS;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u128;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

pub const fn str_eq(l: &str, r: &str) -> bool {
    let (l, r) = (l.as_bytes(), r.as_bytes());

    if l.len() != r.len() {
        return false;
    }

    let mut i = 0;
    while i < l.len() {
        if l[i] != r[i] {
            return false;
        }
        i += 1;
    }

    true
}

pub const fn number_of_digits(n: u128) -> usize {
    if n == 0 {
        return 1;
    }

    let mut len = 0;
    let mut current = n;
    while current % 10 != 0 {
        len += 1;
        current /= 10;
    }
    len
}

pub const fn str_len_from_wrapped_val(ty: Type, v: WrappedValue) -> usize {
    let mut len = ty.len() + 1; // Type!

    let mut current_wrap_val = v;
    while current_wrap_val % 10 != 0 {
        match current_wrap_val % 10 {
            2 => len -= 1, // remove !
            3 => len += 3, // [Type]!
            _ => {}
        }

        current_wrap_val /= 10;
    }

    len
}

const _: () = check_valid_field_args();

pub const fn check_valid_field_args() {
    const BASE: &[(Name, Type, WrappedValue)] = &[("test", "String", 1)];
    const IMPL: &[(Name, Type, WrappedValue)] = &[("test", "String", 12323)];

    const OPENING_BRACKET: &str = "[";
    const CLOSING_BRACKET: &str = "]";
    const BANG: &str = "!";

    const fn find_len() -> usize {
        let mut base_i = 0;
        while base_i < BASE.len() {
            let (base_name, base_type, base_wrap_val) = BASE[base_i];

            let mut impl_i = 0;
            while impl_i < IMPL.len() {
                let (impl_name, impl_type, impl_wrap_val) = IMPL[impl_i];

                if str_eq(base_name, impl_name) {
                    if !(str_eq(base_type, impl_type) && base_wrap_val == impl_wrap_val) {
                        return str_len_from_wrapped_val(impl_type, impl_wrap_val);
                    }
                }
                impl_i += 1;
            }
            base_i += 1;
        }
        0
    }

    let mut base_i = 0;
    while base_i < BASE.len() {
        let (base_name, base_type, base_wrap_val) = BASE[base_i];

        let mut impl_i = 0;
        let mut was_found = false;
        while impl_i < IMPL.len() {
            let (impl_name, impl_type, impl_wrap_val) = IMPL[impl_i];

            if str_eq(base_name, impl_name) {
                if str_eq(base_type, impl_type) && base_wrap_val == impl_wrap_val {
                    was_found = true;
                } else {
                    const IMPL_TYPE_LEN: usize = find_len();
                    let mut impl_type_arr: [u8; IMPL_TYPE_LEN] = [0; IMPL_TYPE_LEN];

                    let mut current_start = 0;
                    let mut current_end = IMPL_TYPE_LEN - 1;
                    let mut current_wrap_val = impl_wrap_val;
                    let mut is_null = false;
                    while current_wrap_val % 10 != 0 {
                        match current_wrap_val % 10 {
                            2 => is_null = true,
                            3 => {
                                if is_null {
                                    let mut i = 0;
                                    while i < OPENING_BRACKET.as_bytes().len() {
                                        impl_type_arr[current_start + i] =
                                            OPENING_BRACKET.as_bytes()[i];
                                        i += 1;
                                    }
                                    current_start += i;
                                    i = 0;
                                    while i < CLOSING_BRACKET.as_bytes().len() {
                                        impl_type_arr[current_end
                                            - CLOSING_BRACKET.as_bytes().len()
                                            + i
                                            + 1] = CLOSING_BRACKET.as_bytes()[i];
                                        i += 1;
                                    }
                                    current_end -= i;
                                } else {
                                    let mut i = 0;
                                    while i < OPENING_BRACKET.as_bytes().len() {
                                        impl_type_arr[current_start + i] =
                                            OPENING_BRACKET.as_bytes()[i];
                                        i += 1;
                                    }
                                    current_start += i;
                                    i = 0;
                                    while i < BANG.as_bytes().len() {
                                        impl_type_arr
                                            [current_end - BANG.as_bytes().len() + i + 1] =
                                            BANG.as_bytes()[i];
                                        i += 1;
                                    }
                                    current_end -= i;
                                    i = 0;
                                    while i < CLOSING_BRACKET.as_bytes().len() {
                                        impl_type_arr[current_end
                                            - CLOSING_BRACKET.as_bytes().len()
                                            + i
                                            + 1] = CLOSING_BRACKET.as_bytes()[i];
                                        i += 1;
                                    }
                                    current_end -= i;
                                }
                                is_null = false;
                            }
                            _ => {}
                        }

                        current_wrap_val /= 10;
                    }

                    let mut i = 0;
                    while i < impl_type.as_bytes().len() {
                        impl_type_arr[current_start + i] = impl_type.as_bytes()[i];
                        i += 1;
                    }
                    i = 0;
                    if !is_null {
                        while i < BANG.as_bytes().len() {
                            impl_type_arr[current_end - BANG.as_bytes().len() + i + 1] =
                                BANG.as_bytes()[i];
                            i += 1;
                        }
                    }

                    let impl_type_str: &str = unsafe { mem::transmute(impl_type_arr.as_slice()) };

                    // TODO: format incorrect field type error.
                    panic!(impl_type_str);
                }
            } else if impl_wrap_val % 10 != 2 {
                // TODO: format field must be nullable error.
                panic!(impl_name);
            }

            impl_i += 1;
        }

        if !was_found {
            // TODO: format field not found error.
            panic!(base_name);
        }

        base_i += 1;
    }
}

pub const fn is_valid_field_args(
    base_interface: &[(Name, Type, WrappedValue)],
    implementation: &[(Name, Type, WrappedValue)],
) -> bool {
    const fn find(
        base_interface: &[(Name, Type, WrappedValue)],
        impl_name: Name,
        impl_ty: Type,
        impl_wrap_val: WrappedValue,
    ) -> bool {
        let mut i = 0;
        while i < base_interface.len() {
            let (base_name, base_ty, base_wrap_val) = base_interface[i];
            if str_eq(impl_name, base_name) {
                return str_eq(base_ty, impl_ty) && impl_wrap_val == base_wrap_val;
            }
            i += 1;
        }
        false
    }

    if base_interface.len() > implementation.len() {
        return false;
    }

    let mut i = 0;
    let mut successfully_implemented_fields = 0;
    while i < implementation.len() {
        let (impl_name, impl_ty, impl_wrap_val) = implementation[i];
        if find(base_interface, impl_name, impl_ty, impl_wrap_val) {
            successfully_implemented_fields += 1;
        } else if impl_wrap_val % 10 != 2 {
            // Not an optional field.
            return false;
        }
        i += 1;
    }

    successfully_implemented_fields == base_interface.len()
}

pub const fn can_be_subtype(ty: WrappedValue, subtype: WrappedValue) -> bool {
    let ty_current = ty % 10;
    let subtype_current = subtype % 10;

    if ty_current == subtype_current {
        if ty_current == 1 {
            true
        } else {
            can_be_subtype(ty / 10, subtype / 10)
        }
    } else if ty_current == 2 {
        can_be_subtype(ty / 10, subtype)
    } else {
        false
    }
}

pub const fn exists(val: Type, arr: Types) -> bool {
    let mut i = 0;
    while i < arr.len() {
        if str_eq(val, arr[i]) {
            return true;
        }
        i += 1;
    }
    false
}

pub const fn is_subtype(
    possible_subtypes: Types,
    wrapped_type: WrappedValue,
    subtype: Type,
    wrapped_subtype: WrappedValue,
) -> bool {
    exists(subtype, possible_subtypes) && can_be_subtype(wrapped_type, wrapped_subtype)
}

/// TODO
pub trait BaseType<S = DefaultScalarValue> {
    const NAME: Type;
}

impl<'a, S, T: BaseType<S> + ?Sized> BaseType<S> for &'a T {
    const NAME: Type = T::NAME;
}

// TODO: Reconsider
impl<S, T: BaseType<S>, Ctx> BaseType<S> for (Ctx, T) {
    const NAME: Type = T::NAME;
}

impl<S, T: BaseType<S>> BaseType<S> for Option<T> {
    const NAME: Type = T::NAME;
}

impl<S, T: BaseType<S>> BaseType<S> for Nullable<T> {
    const NAME: Type = T::NAME;
}

// TODO: Should Err be trait bounded somehow?
impl<S, T: BaseType<S>, E> BaseType<S> for Result<T, E> {
    const NAME: Type = T::NAME;
}

impl<S, T: BaseType<S>> BaseType<S> for Vec<T> {
    const NAME: Type = T::NAME;
}

impl<S, T: BaseType<S>> BaseType<S> for [T] {
    const NAME: Type = T::NAME;
}

impl<S, T: BaseType<S>, const N: usize> BaseType<S> for [T; N] {
    const NAME: Type = T::NAME;
}

impl<S, T: BaseType<S> + ?Sized> BaseType<S> for Box<T> {
    const NAME: Type = T::NAME;
}

impl<S, T: BaseType<S> + ?Sized> BaseType<S> for Arc<T> {
    const NAME: Type = T::NAME;
}

impl<S, T: BaseType<S> + ?Sized> BaseType<S> for Rc<T> {
    const NAME: Type = T::NAME;
}

/// TODO
pub trait BaseSubTypes<S = DefaultScalarValue> {
    const NAMES: Types;
}

impl<'a, S, T: BaseSubTypes<S> + ?Sized> BaseSubTypes<S> for &'a T {
    const NAMES: Types = T::NAMES;
}

// TODO: Reconsider
impl<S, T: BaseSubTypes<S>, Ctx> BaseSubTypes<S> for (Ctx, T) {
    const NAMES: Types = T::NAMES;
}

impl<S, T: BaseSubTypes<S>> BaseSubTypes<S> for Option<T> {
    const NAMES: Types = T::NAMES;
}

impl<S, T: BaseSubTypes<S>> BaseSubTypes<S> for Nullable<T> {
    const NAMES: Types = T::NAMES;
}

// TODO: Should Err be trait bounded somehow?
impl<S, T: BaseSubTypes<S>, E> BaseSubTypes<S> for Result<T, E> {
    const NAMES: Types = T::NAMES;
}

impl<S, T: BaseSubTypes<S>> BaseSubTypes<S> for Vec<T> {
    const NAMES: Types = T::NAMES;
}

impl<S, T: BaseSubTypes<S>> BaseSubTypes<S> for [T] {
    const NAMES: Types = T::NAMES;
}

impl<S, T: BaseSubTypes<S>, const N: usize> BaseSubTypes<S> for [T; N] {
    const NAMES: Types = T::NAMES;
}

impl<S, T: BaseSubTypes<S> + ?Sized> BaseSubTypes<S> for Box<T> {
    const NAMES: Types = T::NAMES;
}

impl<S, T: BaseSubTypes<S> + ?Sized> BaseSubTypes<S> for Arc<T> {
    const NAMES: Types = T::NAMES;
}

impl<S, T: BaseSubTypes<S> + ?Sized> BaseSubTypes<S> for Rc<T> {
    const NAMES: Types = T::NAMES;
}

/// TODO
pub trait WrappedType<S = DefaultScalarValue> {
    /// NonNull  - 1
    /// Nullable - 2
    /// List     - 3
    ///
    /// `[[Int]!] - <Option<Vec<Vec<Option<i32>>>> as WrappedType>::N = 12332`
    const VALUE: u128;
}

impl<'a, S, T: WrappedType<S>> WrappedType<S> for (&'a T::Context, T)
where
    S: ScalarValue,
    T: crate::GraphQLValue<S>,
{
    const VALUE: u128 = T::VALUE;
}

impl<S, T: WrappedType<S>> WrappedType<S> for Option<T> {
    const VALUE: u128 = T::VALUE * 10 + 2;
}

impl<S, T: WrappedType<S>> WrappedType<S> for Nullable<T> {
    const VALUE: u128 = T::VALUE * 10 + 2;
}

// TODO: Should Err be trait bounded somehow?
//       And should `VALUE` be `T::VALUE` or `T::VALUE * 10 + 2`?
impl<S, T: WrappedType<S>, E> WrappedType<S> for Result<T, E> {
    const VALUE: u128 = T::VALUE;
}

impl<S, T: WrappedType<S>> WrappedType<S> for Vec<T> {
    const VALUE: u128 = T::VALUE * 10 + 3;
}

impl<S, T: WrappedType<S>> WrappedType<S> for [T] {
    const VALUE: u128 = T::VALUE * 10 + 3;
}

impl<S, T: WrappedType<S>, const N: usize> WrappedType<S> for [T; N] {
    const VALUE: u128 = T::VALUE * 10 + 3;
}

impl<'a, S, T: WrappedType<S> + ?Sized> WrappedType<S> for &'a T {
    const VALUE: u128 = T::VALUE;
}

impl<S, T: WrappedType<S> + ?Sized> WrappedType<S> for Box<T> {
    const VALUE: u128 = T::VALUE;
}

impl<S, T: WrappedType<S> + ?Sized> WrappedType<S> for Arc<T> {
    const VALUE: u128 = T::VALUE;
}

impl<S, T: WrappedType<S> + ?Sized> WrappedType<S> for Rc<T> {
    const VALUE: u128 = T::VALUE;
}

pub trait Field<S, const N: FieldName> {
    type Context;
    type TypeInfo;
    const TYPE: Type;
    const SUB_TYPES: Types;
    const WRAPPED_VALUE: WrappedValue;
    const ARGUMENTS: &'static [(Name, Type, WrappedValue)];

    fn call(
        &self,
        info: &Self::TypeInfo,
        args: &Arguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S>;
}

pub trait AsyncField<S, const N: FieldName> {
    type Context;
    type TypeInfo;
    const TYPE: Type;
    const SUB_TYPES: Types;
    const WRAPPED_VALUE: WrappedValue;
    const ARGUMENTS: &'static [(Name, Type, WrappedValue)];

    fn call<'b>(
        &'b self,
        info: &'b Self::TypeInfo,
        args: &'b Arguments<S>,
        executor: &'b Executor<Self::Context, S>,
    ) -> BoxFuture<'b, ExecutionResult<S>>;
}
