//! Compile-time reflection of Rust types into GraphQL types.

use crate::behavior;

#[doc(inline)]
pub use self::macros::{
    assert_field, assert_field_args, assert_field_type, assert_has_field, assert_implemented_for,
    assert_interfaces_impls, assert_transitive_impls, const_concat, format_type,
};

/// Name of a [GraphQL type][0] in a GraphQL schema.
///
/// See [`BaseType`] for details.
///
/// [0]: https://spec.graphql.org/October2021#sec-Types
pub type Type = &'static str;

/// List of [`Type`]s.
///
/// See [`BaseSubTypes`] for details.
pub type Types = &'static [Type];

/// Basic reflection of a [GraphQL type][0].
///
/// This trait is transparent to [`Option`], [`Vec`] and other containers, so to
/// fully represent a [GraphQL object][1] we additionally use [`WrappedType`].
///
/// [0]: https://spec.graphql.org/October2021#sec-Types
pub trait BaseType<Behavior: ?Sized = behavior::Standard> {
    /// [`Type`] of this [GraphQL type][0].
    ///
    /// Different Rust types may have the same [`NAME`]. For example, [`String`]
    /// and [`&str`](prim@str) share the `String!` GraphQL [`Type`].
    ///
    /// [`NAME`]: Self::NAME
    /// [0]: https://spec.graphql.org/October2021#sec-Types
    const NAME: Type;
}

/// Reflection of [sub-types][2] of a [GraphQL type][0].
///
/// This trait is transparent to [`Option`], [`Vec`] and other containers.
///
/// [0]: https://spec.graphql.org/October2021#sec-Types
/// [2]: https://spec.graphql.org/October2021#sel-JAHZhCHCDEJDAAAEEFDBtzC
pub trait BaseSubTypes<Behavior: ?Sized = behavior::Standard> {
    /// Sub-[`Types`] of this [GraphQL type][0].
    ///
    /// Contains [at least][2] the [`BaseType::NAME`] of this [GraphQL type][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Types
    /// [2]: https://spec.graphql.org/October2021#sel-JAHZhCHCDEJDAAAEEFDBtzC
    const NAMES: Types;
}

/// Reflection of [GraphQL interfaces][1] implementations for a
/// [GraphQL type][0].
///
/// [0]: https://spec.graphql.org/October2021#sec-Types
/// [1]: https://spec.graphql.org/October2021#sec-Interfaces
pub trait Implements<Behavior: ?Sized = behavior::Standard> {
    /// [`Types`] of the [GraphQL interfaces][1] implemented by this
    /// [GraphQL type][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Types
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    const NAMES: Types;
}

/// Encoded value of a [`WrappedType`] (composed [GraphQL wrapping type][0]).
///
/// See [`WrappedType`] for details.
///
/// [0]: https://spec.graphql.org/October2021#sec-Wrapping-Types
// TODO: Just use `&str`s once they're allowed in `const` generics.
pub type WrappedValue = u128;

/// [`WrappedValue`] encoding helpers.
pub mod wrap {
    use super::WrappedValue;

    /// [`WrappedValue`] of a singular non-nullable [GraphQL type][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Types
    pub const SINGULAR: WrappedValue = 1;

    /// Performs wrapping into a nullable [`WrappedValue`].
    pub const fn nullable(val: WrappedValue) -> WrappedValue {
        val * 10 + 2
    }

    /// Performs wrapping into a list [`WrappedValue`].
    pub const fn list(val: WrappedValue) -> WrappedValue {
        val * 10 + 3
    }
}

/// Reflection of a composed [GraphQL wrapping type][1], encoded in numbers.
///
/// To fully represent a [GraphQL type][0] it's not enough to use [`Type`],
/// because of the [wrapping types][1]. To work around this, a [`WrappedValue`]
/// is used, which is represented via [`u128`] number in the following encoding:
/// - In base case of non-nullable singular [type][0] [`VALUE`] is `1`.
/// - To represent nullability we "append" `2` to the [`VALUE`], so
///   [`Option`]`<`[type][0]`>` has [`VALUE`] of `12`.
/// - To represent a list we "append" `3` to the [`VALUE`], so
///   [`Vec`]`<`[type][0]`>` has [`VALUE`] of `13`.
///
/// Note, that due to Rust type system, the encoding here differs from the one
/// of [GraphQL wrapping types][1], as it takes nullability as wrapping, while
/// GraphQL [does the opposite][1] (takes non-nullability as wrapping).
///
/// This approach allows to uniquely represent any [GraphQL type][0] with a
/// combination of a [`Type`] and a [`WrappedValue`], and even format it via
/// [`format_type!`] macro in a `const` context.
///
/// # Example
///
/// ```rust
/// # use juniper::reflect::{
/// #     format_type, BaseType, Type, WrappedType, WrappedValue,
/// # };
/// #
/// assert_eq!(<Option<i32> as WrappedType>::VALUE, 12);
/// assert_eq!(<Vec<i32> as WrappedType>::VALUE, 13);
/// assert_eq!(<Vec<Option<i32>> as WrappedType>::VALUE, 123);
/// assert_eq!(<Option<Vec<i32>> as WrappedType>::VALUE, 132);
/// assert_eq!(<Option<Vec<Option<i32>>> as WrappedType>::VALUE, 1232);
///
/// const TYPE_STRING: Type = <Option<Vec<Option<String>>> as BaseType>::NAME;
/// const WRAP_VAL_STRING: WrappedValue = <Option<Vec<Option<String>>> as WrappedType>::VALUE;
/// assert_eq!(format_type!(TYPE_STRING, WRAP_VAL_STRING), "[String]");
///
/// const TYPE_STR: Type = <Option<Vec<Option<&str>>> as BaseType>::NAME;
/// const WRAP_VAL_STR: WrappedValue = <Option<Vec<Option<&str>>> as WrappedType>::VALUE;
/// assert_eq!(format_type!(TYPE_STR, WRAP_VAL_STR), "[String]");
/// ```
///
/// [`VALUE`]: Self::VALUE
/// [0]: https://spec.graphql.org/October2021#sec-Types
/// [1]: https://spec.graphql.org/October2021#sec-Wrapping-Types
pub trait WrappedType<Behavior: ?Sized = behavior::Standard> {
    /// [`WrappedValue`] of this this [GraphQL type][0], encoded in a number.
    ///
    /// Use [`format_type!`] macro on this number to represent it as a
    /// human-readable [GraphQL type][0] string.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Types
    const VALUE: WrappedValue;
}

/// Name of a [GraphQL field][0] or a [field argument][1].
///
/// See [`Fields`] for details.
///
/// [0]: https://spec.graphql.org/October2021#sec-Language.Fields
/// [1]: https://spec.graphql.org/October2021#sec-Language.Arguments
pub type Name = &'static str;

/// List of [`Name`]s.
///
/// See [`Fields`] for details.
pub type Names = &'static [Name];

/// Reflection of [fields][0] for a [GraphQL object][1] or an [interface][2].
///
/// [0]: https://spec.graphql.org/October2021#sec-Language.Fields
/// [1]: https://spec.graphql.org/October2021#sec-Objects
/// [2]: https://spec.graphql.org/October2021#sec-Interfaces
pub trait Fields<Behavior: ?Sized = behavior::Standard> {
    /// [`Names`] of this [GraphQL object][1]/[interface][2] [fields][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Language.Fields
    /// [1]: https://spec.graphql.org/October2021#sec-Objects
    /// [2]: https://spec.graphql.org/October2021#sec-Interfaces
    const NAMES: Names;
}

/// [GraphQL field argument][0], represented as its [`Name`], [`Type`] and
/// [`WrappedValue`].
///
/// See [`Field`] for details.
///
/// [0]: https://spec.graphql.org/October2021#sec-Language.Arguments
pub type Argument = (Name, Type, WrappedValue);

/// List of [`Argument`]s.
///
/// See [`Field`] for details.
pub type Arguments = &'static [(Name, Type, WrappedValue)];

/// Alias for a `const`-hashed [`Name`] used in a `const` context.
// TODO: Just use `&str`s once they're allowed in `const` generics.
pub type FieldName = u128;

/// Reflection of a single [GraphQL field][0].
///
/// [0]: https://spec.graphql.org/October2021#sec-Language.Fields
pub trait Field<const N: FieldName, Behavior: ?Sized = behavior::Standard> {
    /// [`Type`] of this [GraphQL field][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Language.Fields
    const TYPE: Type;

    /// [Sub-types][1] this [GraphQL field][0] is coercible into.
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Language.Fields
    /// [1]: BaseSubTypes
    const SUB_TYPES: Types;

    /// [`WrappedValue`] of this [GraphQL field][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Language.Fields
    const WRAPPED_VALUE: WrappedValue;

    /// [`Arguments`] of this [GraphQL field][0] .
    ///
    /// [0]: https://spec.graphql.org/October2021#sec-Language.Fields
    const ARGUMENTS: Arguments;
}

/// Non-cryptographic hash with good dispersion to use as a [`str`](prim@str) in
/// `const` generics. See [spec] for more info.
///
/// [spec]: https://datatracker.ietf.org/doc/html/draft-eastlake-fnv-17.html
#[must_use]
pub const fn fnv1a128(str: Name) -> FieldName {
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

/// Length __in bytes__ of the [`format_type!`] macro result.
#[must_use]
pub const fn type_len_with_wrapped_val(ty: Type, val: WrappedValue) -> usize {
    let mut len = ty.as_bytes().len() + "!".as_bytes().len(); // Type!

    let mut curr = val;
    while curr % 10 != 0 {
        match curr % 10 {
            2 => len -= "!".as_bytes().len(),   // remove !
            3 => len += "[]!".as_bytes().len(), // [Type]!
            _ => {}
        }
        curr /= 10;
    }

    len
}

/// Checks whether the specified `subtype` [GraphQL type][0] represents a
/// [sub-type][1] of the specified `supertype`, basing on the [`WrappedType`]
/// encoding.
///
/// To fully determine the [sub-typing][1] relation the [`Type`] should be one
/// of the [`BaseSubTypes::NAMES`].
///
/// [0]: https://spec.graphql.org/October2021#sec-Types
/// [1]: https://spec.graphql.org/October2021#sel-JAHZhCHCDEJDAAAEEFDBtzC
#[must_use]
pub const fn can_be_subtype(supertype: WrappedValue, subtype: WrappedValue) -> bool {
    let super_curr = supertype % 10;
    let sub_curr = subtype % 10;

    if super_curr == sub_curr {
        if super_curr == 1 {
            true
        } else {
            can_be_subtype(supertype / 10, subtype / 10)
        }
    } else if super_curr == 2 {
        can_be_subtype(supertype / 10, subtype)
    } else {
        false
    }
}

/// Checks whether the given `val` exists in the given `arr`.
// TODO: Remove once `slice::contains()` method is allowed in `const` context.
#[must_use]
pub const fn str_exists_in_arr(val: &str, arr: &[&str]) -> bool {
    let mut i = 0;
    while i < arr.len() {
        if str_eq(val, arr[i]) {
            return true;
        }
        i += 1;
    }
    false
}

/// Compares strings in a `const` context.
///
/// As there is no `const impl Trait` and `l == r` calls [`Eq`], we have to
/// provide a custom comparison function.
///
/// [`Eq`]: std::cmp::Eq
// TODO: Remove once `Eq` trait is allowed in `const` context.
#[must_use]
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

mod macros {
    /// Asserts that `#[graphql::interface(for = ...)]` attribute has all the
    /// types referencing this interface in the `impl = ...` attribute argument.
    ///
    /// Symmetrical to [`assert_interfaces_impls!`].
    #[macro_export]
    macro_rules! reflect_assert_implemented_for {
        ($behavior: ty, $implementor: ty $(, $interfaces: ty)* $(,)?) => {
            const _: () = { $({
                let is_present = $crate::reflect::str_exists_in_arr(
                    <$implementor as $crate::reflect::BaseType<$behavior>>::NAME,
                    <$interfaces as $crate::reflect::BaseSubTypes<$behavior>>::NAMES,
                );
                if !is_present {
                    const MSG: &str = $crate::reflect::const_concat!(
                        "Failed to implement interface `",
                        <$interfaces as $crate::reflect::BaseType<$behavior>>::NAME,
                        "` on `",
                        <$implementor as $crate::reflect::BaseType<$behavior>>::NAME,
                        "`: missing implementer reference in interface's `for` attribute.",
                    );
                    ::std::panic!("{}", MSG);
                }
            })* };
        };
    }

    /// Asserts that `impl = ...` attribute argument has all the interfaces
    /// referencing this type in `#[graphql::interface(for = ...)]` attribute.
    ///
    /// Symmetrical to [`assert_implemented_for!`].
    #[macro_export]
    macro_rules! reflect_assert_interfaces_impls {
        ($behavior: ty, $interface: ty $(, $implementers: ty)* $(,)?) => {
            const _: () = { $({
                let is_present = $crate::reflect::str_exists_in_arr(
                    <$interface as $crate::reflect::BaseType<$behavior>>::NAME,
                    <$implementers as $crate::reflect::Implements<$behavior>>::NAMES,
                );
                if !is_present {
                    const MSG: &str = $crate::reflect::const_concat!(
                        "Failed to implement interface `",
                        <$interface as $crate::reflect::BaseType<$behavior>>::NAME,
                        "` on `",
                        <$implementers as $crate::reflect::BaseType<$behavior>>::NAME,
                        "`: missing interface reference in implementer's `impl` attribute.",
                    );
                    ::std::panic!("{}", MSG);
                }
            })* };
        };
    }

    /// Asserts that all [transitive interfaces][0] (the ones implemented by the
    /// `$interface`) are also implemented by the `$implementor`.
    ///
    /// [0]: https://spec.graphql.org/October2021#sel-FAHbhBHCAACGB35P
    #[macro_export]
    macro_rules! reflect_assert_transitive_impls {
        ($behavior: ty, $interface: ty, $implementor: ty $(, $transitive: ty)* $(,)?) => {
            const _: () = { $({
                let is_present = $crate::reflect::str_exists_in_arr(
                    <$implementor as $crate::reflect::BaseType<$behavior>>::NAME,
                    <$transitive as $crate::reflect::BaseSubTypes<$behavior>>::NAMES,
                );
                if !is_present {
                    const MSG: &str = $crate::reflect::const_concat!(
                        "Failed to implement interface `",
                        <$interface as $crate::reflect::BaseType<$behavior>>::NAME,
                        "` on `",
                        <$implementor as $crate::reflect::BaseType<$behavior>>::NAME,
                        "`: missing `impl = ` for transitive interface `",
                        <$transitive as $crate::reflect::BaseType<$behavior>>::NAME,
                        "` on `",
                        <$implementor as $crate::reflect::BaseType<$behavior>>::NAME,
                        "`.",
                    );
                    ::std::panic!("{}", MSG);
                }
            })* };
        };
    }

    /// Asserts validness of [`Field`] [`Arguments`] and its returned [`Type`].
    ///
    /// This assertion is a combination of [`assert_field_type!`] and
    /// [`assert_field_args!`].
    ///
    /// See [spec][0] for assertion algorithm details.
    ///
    /// [`Arguments`]: super::Arguments
    /// [`Field`]: super::Field
    /// [`Type`]: super::Type
    /// [0]: https://spec.graphql.org/October2021#IsValidImplementation()
    #[macro_export]
    macro_rules! reflect_assert_field {
        (
            $base_ty: ty,
            $impl_ty: ty,
            $behavior: ty,
            $field_name: expr $(,)?
        ) => {
            $crate::reflect::assert_field_type!($base_ty, $impl_ty, $behavior, $field_name);
            $crate::reflect::assert_field_args!($base_ty, $impl_ty, $behavior, $field_name);
        };
    }

    /// Asserts validness of a [`Field`] type.
    ///
    /// See [spec][0] for assertion algorithm details.
    ///
    /// [`Field`]: super::Field
    /// [0]: https://spec.graphql.org/October2021#IsValidImplementationFieldType()
    #[macro_export]
    macro_rules! reflect_assert_field_type {
        (
            $base_ty: ty,
            $impl_ty: ty,
            $behavior: ty,
            $field_name: expr $(,)?
        ) => {
            const _: () = {
                const BASE_TY: $crate::reflect::Type =
                    <$base_ty as $crate::reflect::BaseType<$behavior>>::NAME;
                const IMPL_TY: $crate::reflect::Type =
                    <$impl_ty as $crate::reflect::BaseType<$behavior>>::NAME;
                const ERR_PREFIX: &str = $crate::reflect::const_concat!(
                    "Failed to implement interface `",
                    BASE_TY,
                    "` on `",
                    IMPL_TY,
                    "`: ",
                );

                const FIELD_NAME: $crate::reflect::Name = $field_name;
                const FIELD_NAME_HASH: $crate::reflect::FieldName =
                    $crate::reflect::fnv1a128(FIELD_NAME);

                $crate::reflect::assert_has_field!(
                    FIELD_NAME, $base_ty, $behavior, ERR_PREFIX,
                );
                $crate::reflect::assert_has_field!(
                    FIELD_NAME, $impl_ty, $behavior, ERR_PREFIX,
                );

                const BASE_RETURN_WRAPPED_VAL: $crate::reflect::WrappedValue =
                    <$base_ty as $crate::reflect::Field<
                        FIELD_NAME_HASH, $behavior,
                    >>::WRAPPED_VALUE;
                const IMPL_RETURN_WRAPPED_VAL: $crate::reflect::WrappedValue =
                    <$impl_ty as $crate::reflect::Field<
                        FIELD_NAME_HASH, $behavior,
                    >>::WRAPPED_VALUE;

                const BASE_RETURN_TY: $crate::reflect::Type =
                    <$base_ty as $crate::reflect::Field<
                        FIELD_NAME_HASH, $behavior,
                    >>::TYPE;
                const IMPL_RETURN_TY: $crate::reflect::Type =
                    <$impl_ty as $crate::reflect::Field<
                        FIELD_NAME_HASH, $behavior,
                    >>::TYPE;

                const BASE_RETURN_SUB_TYPES: $crate::reflect::Types =
                    <$base_ty as $crate::reflect::Field<
                        FIELD_NAME_HASH, $behavior,
                    >>::SUB_TYPES;

                let is_subtype = $crate::reflect::str_exists_in_arr(
                    IMPL_RETURN_TY, BASE_RETURN_SUB_TYPES,
                ) && $crate::reflect::can_be_subtype(
                    BASE_RETURN_WRAPPED_VAL, IMPL_RETURN_WRAPPED_VAL,
                );
                if !is_subtype {
                    const MSG: &str = $crate::reflect::const_concat!(
                        ERR_PREFIX,
                        "Field `",
                        FIELD_NAME,
                        "`: implementor is expected to return a subtype of interface's return object: `",
                        $crate::reflect::format_type!(IMPL_RETURN_TY, IMPL_RETURN_WRAPPED_VAL),
                        "` is not a subtype of `",
                        $crate::reflect::format_type!(BASE_RETURN_TY, BASE_RETURN_WRAPPED_VAL),
                        "`.",
                    );
                    ::std::panic!("{}", MSG);
                }
            };
        };
    }

    /// Asserts validness of a [`Field`] arguments.
    ///
    /// See [spec][0] for assertion algorithm details.
    ///
    /// [`Field`]: super::Field
    /// [0]: https://spec.graphql.org/October2021#sel-IAHZhCHCDEEFAAADHD8Cxob
    #[macro_export]
    macro_rules! reflect_assert_field_args {
        (
            $base_ty: ty,
            $impl_ty: ty,
            $behavior: ty,
            $field_name: expr $(,)?
        ) => {
            const _: () = {
                const BASE_TY: $crate::reflect::Type =
                    <$base_ty as $crate::reflect::BaseType<$behavior>>::NAME;
                const IMPL_TY: $crate::reflect::Type =
                    <$impl_ty as $crate::reflect::BaseType<$behavior>>::NAME;
                const ERR_PREFIX: &str = $crate::reflect::const_concat!(
                    "Failed to implement interface `",
                    BASE_TY,
                    "` on `",
                    IMPL_TY,
                    "`: ",
                );

                const FIELD_NAME: $crate::reflect::Name = $field_name;
                const FIELD_NAME_HASH: $crate::reflect::FieldName =
                    $crate::reflect::fnv1a128(FIELD_NAME);

                $crate::reflect::assert_has_field!(FIELD_NAME, $base_ty, $behavior, ERR_PREFIX);
                $crate::reflect::assert_has_field!(FIELD_NAME, $impl_ty, $behavior, ERR_PREFIX);

                const BASE_ARGS: ::juniper::reflect::Arguments =
                    <$base_ty as $crate::reflect::Field<FIELD_NAME_HASH, $behavior>>::ARGUMENTS;
                const IMPL_ARGS: ::juniper::reflect::Arguments =
                    <$impl_ty as $crate::reflect::Field<FIELD_NAME_HASH, $behavior>>::ARGUMENTS;

                struct Error {
                    cause: Cause,
                    base: ::juniper::reflect::Argument,
                    implementation: ::juniper::reflect::Argument,
                }

                enum Cause {
                    RequiredField,
                    AdditionalNonNullableField,
                    TypeMismatch,
                }

                const fn unwrap_error(v: ::std::result::Result<(), Error>) -> Error {
                    match v {
                        // Unfortunately, we cannot use `unreachable!()` here,
                        // as this branch will be executed either way.
                        Ok(()) => Error {
                            cause: Cause::RequiredField,
                            base: ("unreachable", "unreachable", 1),
                            implementation: ("unreachable", "unreachable", 1),
                        },
                        Err(e) => e,
                    }
                }

                const fn check() -> Result<(), Error> {
                    let mut base_i = 0;
                    while base_i < BASE_ARGS.len() {
                        let (base_name, base_type, base_wrap_val) = BASE_ARGS[base_i];

                        let mut impl_i = 0;
                        let mut was_found = false;
                        while impl_i < IMPL_ARGS.len() {
                            let (impl_name, impl_type, impl_wrap_val) = IMPL_ARGS[impl_i];

                            if $crate::reflect::str_eq(base_name, impl_name) {
                                if $crate::reflect::str_eq(base_type, impl_type)
                                    && base_wrap_val == impl_wrap_val
                                {
                                    was_found = true;
                                    break;
                                } else {
                                    return Err(Error {
                                        cause: Cause::TypeMismatch,
                                        base: (base_name, base_type, base_wrap_val),
                                        implementation: (impl_name, impl_type, impl_wrap_val),
                                    });
                                }
                            }

                            impl_i += 1;
                        }

                        if !was_found {
                            return Err(Error {
                                cause: Cause::RequiredField,
                                base: (base_name, base_type, base_wrap_val),
                                implementation: (base_name, base_type, base_wrap_val),
                            });
                        }

                        base_i += 1;
                    }

                    let mut impl_i = 0;
                    while impl_i < IMPL_ARGS.len() {
                        let (impl_name, impl_type, impl_wrapped_val) = IMPL_ARGS[impl_i];
                        impl_i += 1;

                        if impl_wrapped_val % 10 == 2 {
                            continue;
                        }

                        let mut base_i = 0;
                        let mut was_found = false;
                        while base_i < BASE_ARGS.len() {
                            let (base_name, _, _) = BASE_ARGS[base_i];
                            if $crate::reflect::str_eq(base_name, impl_name) {
                                was_found = true;
                                break;
                            }
                            base_i += 1;
                        }
                        if !was_found {
                            return Err(Error {
                                cause: Cause::AdditionalNonNullableField,
                                base: (impl_name, impl_type, impl_wrapped_val),
                                implementation: (impl_name, impl_type, impl_wrapped_val),
                            });
                        }
                    }

                    Ok(())
                }

                const RES: ::std::result::Result<(), Error> = check();
                if RES.is_err() {
                    const ERROR: Error = unwrap_error(RES);

                    const BASE_ARG_NAME: $crate::reflect::Name = ERROR.base.0;
                    const IMPL_ARG_NAME: $crate::reflect::Name = ERROR.implementation.0;

                    const BASE_TYPE_FORMATTED: &str =
                        $crate::reflect::format_type!(ERROR.base.1, ERROR.base.2,);
                    const IMPL_TYPE_FORMATTED: &str = $crate::reflect::format_type!(
                        ERROR.implementation.1,
                        ERROR.implementation.2,
                    );

                    const MSG: &str = match ERROR.cause {
                        Cause::TypeMismatch => {
                            $crate::reflect::const_concat!(
                                "Argument `",
                                BASE_ARG_NAME,
                                "`: expected type `",
                                BASE_TYPE_FORMATTED,
                                "`, found: `",
                                IMPL_TYPE_FORMATTED,
                                "`.",
                            )
                        }
                        Cause::RequiredField => {
                            $crate::reflect::const_concat!(
                                "Argument `",
                                BASE_ARG_NAME,
                                "` of type `",
                                BASE_TYPE_FORMATTED,
                                "` was expected, but not found.",
                            )
                        }
                        Cause::AdditionalNonNullableField => {
                            $crate::reflect::const_concat!(
                                "Argument `",
                                IMPL_ARG_NAME,
                                "` of type `",
                                IMPL_TYPE_FORMATTED,
                                "` isn't present on the interface and so has to be nullable.",
                            )
                        }
                    };
                    const ERROR_MSG: &str = $crate::reflect::const_concat!(
                        ERR_PREFIX, "Field `", FIELD_NAME, "`: ", MSG,
                    );
                    ::std::panic!("{}", ERROR_MSG);
                }
            };
        };
    }

    /// Ensures that the given `$impl_ty` has the specified [`Field`].
    ///
    /// [`Field`]: super::Field
    /// [`fnv1a128`]: super::fnv1a128
    #[macro_export]
    macro_rules! reflect_assert_has_field {
        (
            $field_name: expr,
            $impl_ty: ty,
            $behavior: ty
            $(, $prefix: expr)? $(,)?
        ) => {{
            let exists = $crate::reflect::str_exists_in_arr(
                $field_name,
                <$impl_ty as $crate::reflect::Fields<$behavior>>::NAMES,
            );
            if !exists {
                const MSG: &str = $crate::reflect::const_concat!(
                    $($prefix,)?
                    "Field `",
                    $field_name,
                    "` isn't implemented on `",
                    <$impl_ty as $crate::reflect::BaseType<$behavior>>::NAME,
                    "`."
                );
                ::std::panic!("{}", MSG);
            }
        }};
    }

    /// Concatenates `const` [`str`](prim@str)s in a `const` context.
    #[macro_export]
    macro_rules! reflect_const_concat {
        ($($s:expr),* $(,)?) => {{
            const LEN: usize = 0 $(+ $s.as_bytes().len())*;
            const CNT: usize = [$($s),*].len();
            const fn concat(input: [&str; CNT]) -> [u8; LEN] {
                let mut bytes = [0; LEN];
                let (mut i, mut byte) = (0, 0);
                while i < CNT {
                    let mut b = 0;
                    while b < input[i].len() {
                        bytes[byte] = input[i].as_bytes()[b];
                        byte += 1;
                        b += 1;
                    }
                    i += 1;
                }
                bytes
            }
            const CON: [u8; LEN] = concat([$($s),*]);

            // TODO: Use `str::from_utf8()` once it becomes `const`.
            // SAFETY: This is safe, as we concatenate multiple UTF-8 strings
            //         one after another byte-by-byte.
            #[allow(unsafe_code)]
            unsafe { ::std::str::from_utf8_unchecked(&CON) }
        }};
    }

    /// Formats the given [`Type`] and [`WrappedValue`] into a human-readable
    /// GraphQL type name string.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use juniper::reflect::format_type;
    /// #
    /// assert_eq!(format_type!("String", 123), "[String]!");
    /// assert_eq!(format_type!("🦀", 123), "[🦀]!");
    /// ```
    ///
    /// [`Type`]: super::Type
    /// [`WrappedValue`]: super::WrappedValue
    #[macro_export]
    macro_rules! reflect_format_type {
        ($ty: expr, $wrapped_value: expr $(,)?) => {{
            const TYPE: ($crate::reflect::Type, $crate::reflect::WrappedValue) =
                ($ty, $wrapped_value);
            const RES_LEN: usize = $crate::reflect::type_len_with_wrapped_val(TYPE.0, TYPE.1);

            const OPENING_BRACKET: &str = "[";
            const CLOSING_BRACKET: &str = "]";
            const BANG: &str = "!";

            const fn format_type_arr() -> [u8; RES_LEN] {
                let (ty, wrap_val) = TYPE;
                let mut type_arr: [u8; RES_LEN] = [0; RES_LEN];

                let mut current_start = 0;
                let mut current_end = RES_LEN - 1;
                let mut current_wrap_val = wrap_val;
                let mut is_null = false;
                while current_wrap_val % 10 != 0 {
                    match current_wrap_val % 10 {
                        2 => is_null = true, // Skips writing `BANG` later.
                        3 => {
                            // Write `OPENING_BRACKET` at `current_start`.
                            let mut i = 0;
                            while i < OPENING_BRACKET.as_bytes().len() {
                                type_arr[current_start + i] = OPENING_BRACKET.as_bytes()[i];
                                i += 1;
                            }
                            current_start += i;
                            if !is_null {
                                // Write `BANG` at `current_end`.
                                i = 0;
                                while i < BANG.as_bytes().len() {
                                    type_arr[current_end - BANG.as_bytes().len() + i + 1] =
                                        BANG.as_bytes()[i];
                                    i += 1;
                                }
                                current_end -= i;
                            }
                            // Write `CLOSING_BRACKET` at `current_end`.
                            i = 0;
                            while i < CLOSING_BRACKET.as_bytes().len() {
                                type_arr[current_end - CLOSING_BRACKET.as_bytes().len() + i + 1] =
                                    CLOSING_BRACKET.as_bytes()[i];
                                i += 1;
                            }
                            current_end -= i;
                            is_null = false;
                        }
                        _ => {}
                    }

                    current_wrap_val /= 10;
                }

                // Writes `Type` at `current_start`.
                let mut i = 0;
                while i < ty.as_bytes().len() {
                    type_arr[current_start + i] = ty.as_bytes()[i];
                    i += 1;
                }
                i = 0;
                if !is_null {
                    // Writes `BANG` at `current_end`.
                    while i < BANG.as_bytes().len() {
                        type_arr[current_end - BANG.as_bytes().len() + i + 1] = BANG.as_bytes()[i];
                        i += 1;
                    }
                }

                type_arr
            }

            const TYPE_ARR: [u8; RES_LEN] = format_type_arr();

            // TODO: Use `str::from_utf8()` once it becomes `const`.
            // SAFETY: This is safe, as we concatenate multiple UTF-8 strings one
            //         after another byte-by-byte.
            #[allow(unsafe_code)]
            const TYPE_FORMATTED: &str =
                unsafe { ::std::str::from_utf8_unchecked(TYPE_ARR.as_slice()) };
            TYPE_FORMATTED
        }};
    }

    #[doc(inline)]
    pub use {
        reflect_assert_field as assert_field, reflect_assert_field_args as assert_field_args,
        reflect_assert_field_type as assert_field_type,
        reflect_assert_has_field as assert_has_field,
        reflect_assert_implemented_for as assert_implemented_for,
        reflect_assert_interfaces_impls as assert_interfaces_impls,
        reflect_assert_transitive_impls as assert_transitive_impls,
        reflect_const_concat as const_concat, reflect_format_type as format_type,
    };
}
