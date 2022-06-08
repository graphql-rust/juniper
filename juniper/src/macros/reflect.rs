//! Compile-time reflection of Rust types into GraphQL types.

use futures::future::BoxFuture;

use crate::{
    reflect::{
        can_be_subtype, fnv1a128, str_eq, str_exists_in_arr, type_len_with_wrapped_val, wrap,
        Argument, Arguments, FieldName, Name, Names, Type, Types, WrappedValue,
    },
    Arguments as FieldArguments, ExecutionResult, Executor, GraphQLValue, ScalarValue, Nullable,
};

/// Naming of a [GraphQL object][1], [scalar][2] or [interface][3] [`Type`].
///
/// This trait is transparent to [`Option`], [`Vec`] and other containers, so to
/// fully represent a [GraphQL object][1] we additionally use [`WrappedType`].
///
/// Different Rust types may have the same [`NAME`]. For example, [`String`] and
/// `&`[`str`](prim@str) share `String!` GraphQL type.
///
/// [`NAME`]: Self::NAME
/// [1]: https://spec.graphql.org/October2021#sec-Objects
/// [2]: https://spec.graphql.org/October2021#sec-Scalars
/// [3]: https://spec.graphql.org/October2021#sec-Interfaces
pub trait BaseType<S> {
    /// [`Type`] of the [GraphQL object][1], [scalar][2] or [interface][3].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Objects
    /// [2]: https://spec.graphql.org/October2021#sec-Scalars
    /// [3]: https://spec.graphql.org/October2021#sec-Interfaces
    const NAME: Type;
}

impl<'a, S, T: BaseType<S> + ?Sized> BaseType<S> for &'a T {
    const NAME: Type = T::NAME;
}

impl<'ctx, S, T> BaseType<S> for (&'ctx T::Context, T)
where
    S: ScalarValue,
    T: BaseType<S> + GraphQLValue<S>,
{
    const NAME: Type = T::NAME;
}

impl<S, T: BaseType<S>> BaseType<S> for Option<T> {
    const NAME: Type = T::NAME;
}

impl<S, T: BaseType<S>> BaseType<S> for Nullable<T> {
    const NAME: Type = T::NAME;
}

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

/// [Sub-types][2] of a [GraphQL object][1].
///
/// This trait is transparent to [`Option`], [`Vec`] and other containers.
///
/// [1]: https://spec.graphql.org/October2021#sec-Objects
/// [2]: https://spec.graphql.org/October2021#sel-JAHZhCHCDEJDAAAEEFDBtzC
pub trait BaseSubTypes<S> {
    /// Sub-[`Types`] of the [GraphQL object][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Objects
    const NAMES: Types;
}

impl<'a, S, T: BaseSubTypes<S> + ?Sized> BaseSubTypes<S> for &'a T {
    const NAMES: Types = T::NAMES;
}

impl<'ctx, S, T> BaseSubTypes<S> for (&'ctx T::Context, T)
where
    S: ScalarValue,
    T: BaseSubTypes<S> + GraphQLValue<S>,
{
    const NAMES: Types = T::NAMES;
}

impl<S, T: BaseSubTypes<S>> BaseSubTypes<S> for Option<T> {
    const NAMES: Types = T::NAMES;
}

impl<S, T: BaseSubTypes<S>> BaseSubTypes<S> for Nullable<T> {
    const NAMES: Types = T::NAMES;
}

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

// TODO: Just use `&str`s once they're allowed in `const` generics.
/// Encoding of a composed GraphQL type in numbers.
///
/// To fully represent a [GraphQL object][1] it's not enough to use [`Type`],
/// because of the [wrapping types][2]. To work around this we use a
/// [`WrappedValue`] which is represented via [`u128`] number in the following
/// encoding:
/// - In base case of non-nullable singular [object][1] [`VALUE`] is `1`.
/// - To represent nullability we "append" `2` to the [`VALUE`], so
///   [`Option`]`<`[object][1]`>` has [`VALUE`] of `12`.
/// - To represent list we "append" `3` to the [`VALUE`], so
///   [`Vec`]`<`[object][1]`>` has [`VALUE`] of `13`.
///
/// This approach allows us to uniquely represent any [GraphQL object][1] with a
/// combination of [`Type`] and [`WrappedValue`] and even format it via
/// [`format_type!`] macro in a `const` context.
///
/// # Examples
///
/// ```rust
/// # use juniper::{
/// #     reflect::format_type,
/// #     macros::reflect::{WrappedType, BaseType, WrappedValue, Type},
/// #     DefaultScalarValue,
/// # };
/// #
/// assert_eq!(<Option<i32> as WrappedType<DefaultScalarValue>>::VALUE, 12);
/// assert_eq!(<Vec<i32> as WrappedType<DefaultScalarValue>>::VALUE, 13);
/// assert_eq!(<Vec<Option<i32>> as WrappedType<DefaultScalarValue>>::VALUE, 123);
/// assert_eq!(<Option<Vec<i32>> as WrappedType<DefaultScalarValue>>::VALUE, 132);
/// assert_eq!(<Option<Vec<Option<i32>>> as WrappedType<DefaultScalarValue>>::VALUE, 1232);
///
/// const TYPE_STRING: Type = <Option<Vec<Option<String>>> as BaseType<DefaultScalarValue>>::NAME;
/// const WRAP_VAL_STRING: WrappedValue = <Option<Vec<Option<String>>> as WrappedType<DefaultScalarValue>>::VALUE;
/// assert_eq!(format_type!(TYPE_STRING, WRAP_VAL_STRING), "[String]");
///
/// const TYPE_STR: Type = <Option<Vec<Option<&str>>> as BaseType<DefaultScalarValue>>::NAME;
/// const WRAP_VAL_STR: WrappedValue = <Option<Vec<Option<&str>>> as WrappedType<DefaultScalarValue>>::VALUE;
/// assert_eq!(format_type!(TYPE_STR, WRAP_VAL_STR), "[String]");
/// ```
///
/// [`VALUE`]: Self::VALUE
/// [1]: https://spec.graphql.org/October2021#sec-Objects
/// [2]: https://spec.graphql.org/October2021#sec-Wrapping-Types
pub trait WrappedType<S> {
    /// [`WrappedValue`] of this type.
    const VALUE: WrappedValue;
}

impl<'ctx, S, T: WrappedType<S>> WrappedType<S> for (&'ctx T::Context, T)
where
    S: ScalarValue,
    T: GraphQLValue<S>,
{
    const VALUE: u128 = T::VALUE;
}

impl<S, T: WrappedType<S>> WrappedType<S> for Option<T> {
    const VALUE: u128 = T::VALUE * 10 + 2;
}

impl<S, T: WrappedType<S>> WrappedType<S> for Nullable<T> {
    const VALUE: u128 = T::VALUE * 10 + 2;
}

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

/// [GraphQL object][1] or [interface][2] [field arguments][3] [`Names`].
///
/// [1]: https://spec.graphql.org/October2021#sec-Objects
/// [2]: https://spec.graphql.org/October2021#sec-Interfaces
/// [3]: https://spec.graphql.org/October2021#sec-Language.Arguments
pub trait Fields<S> {
    /// [`Names`] of the [GraphQL object][1] or [interface][2]
    /// [field arguments][3].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Objects
    /// [2]: https://spec.graphql.org/October2021#sec-Interfaces
    /// [3]: https://spec.graphql.org/October2021#sec-Language.Arguments
    const NAMES: Names;
}

/// [`Types`] of the [GraphQL interfaces][1] implemented by this type.
///
/// [1]: https://spec.graphql.org/October2021#sec-Interfaces
pub trait Implements<S> {
    /// [`Types`] of the [GraphQL interfaces][1] implemented by this type.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Interfaces
    const NAMES: Types;
}

/// Stores meta information of a [GraphQL field][1]:
/// - [`Context`] and [`TypeInfo`].
/// - Return type's [`TYPE`], [`SUB_TYPES`] and [`WRAPPED_VALUE`].
/// - [`ARGUMENTS`].
///
/// [`ARGUMENTS`]: Self::ARGUMENTS
/// [`Context`]: Self::Context
/// [`SUB_TYPES`]: Self::SUB_TYPES
/// [`TYPE`]: Self::TYPE
/// [`TypeInfo`]: Self::TypeInfo
/// [`WRAPPED_VALUE`]: Self::WRAPPED_VALUE
/// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
pub trait FieldMeta<S, const N: FieldName> {
    /// [`GraphQLValue::Context`] of this [field][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    type Context;

    /// [`GraphQLValue::TypeInfo`] of this [GraphQL field][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    type TypeInfo;

    /// [`Types`] of [GraphQL field's][1] return type.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    const TYPE: Type;

    /// [Sub-types][1] of [GraphQL field's][2] return type.
    ///
    /// [1]: BaseSubTypes
    /// [2]: https://spec.graphql.org/October2021#sec-Language.Fields
    const SUB_TYPES: Types;

    /// [`WrappedValue`] of [GraphQL field's][1] return type.
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    const WRAPPED_VALUE: WrappedValue;

    /// [GraphQL field's][1] [`Arguments`].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
    const ARGUMENTS: Arguments;
}

/// Synchronous field of a [GraphQL object][1] or [interface][2].
///
/// [1]: https://spec.graphql.org/October2021#sec-Objects
/// [2]: https://spec.graphql.org/October2021#sec-Interfaces
pub trait Field<S, const N: FieldName>: FieldMeta<S, N> {
    /// Resolves the [`Value`] of this synchronous [`Field`].
    ///
    /// The `arguments` object contains all the specified arguments, with the
    /// default values being substituted for the ones not provided by the query.
    ///
    /// The `executor` can be used to drive selections into sub-[objects][1].
    ///
    /// [`Value`]: crate::Value
    /// [1]: https://spec.graphql.org/October2021#sec-Objects
    fn call(
        &self,
        info: &Self::TypeInfo,
        args: &FieldArguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S>;
}

/// Asynchronous field of a GraphQL [object][1] or [interface][2].
///
/// [1]: https://spec.graphql.org/October2021#sec-Objects
/// [2]: https://spec.graphql.org/October2021#sec-Interfaces
pub trait AsyncField<S, const N: FieldName>: FieldMeta<S, N> {
    /// Resolves the [`Value`] of this asynchronous [`AsyncField`].
    ///
    /// The `arguments` object contains all the specified arguments, with the
    /// default values being substituted for the ones not provided by the query.
    ///
    /// The `executor` can be used to drive selections into sub-[objects][1].
    ///
    /// [1]: https://spec.graphql.org/October2021#sec-Objects
    fn call<'b>(
        &'b self,
        info: &'b Self::TypeInfo,
        args: &'b FieldArguments<S>,
        executor: &'b Executor<Self::Context, S>,
    ) -> BoxFuture<'b, ExecutionResult<S>>;
}

/// Asserts that `#[graphql_interface(for = ...)]` has all the types referencing
/// this interface in the `impl = ...` attribute argument.
///
/// Symmetrical to [`assert_interfaces_impls!`].
#[macro_export]
macro_rules! assert_implemented_for {
    ($scalar: ty, $implementor: ty $(, $interfaces: ty)* $(,)?) => {
        const _: () = {
            $({
                let is_present = $crate::macros::reflect::str_exists_in_arr(
                    <$implementor as ::juniper::macros::reflect::BaseType<$scalar>>::NAME,
                    <$interfaces as ::juniper::macros::reflect::BaseSubTypes<$scalar>>::NAMES,
                );
                if !is_present {
                    const MSG: &str = $crate::reflect::const_concat!(
                        "Failed to implement interface `",
                        <$interfaces as $crate::macros::reflect::BaseType<$scalar>>::NAME,
                        "` on `",
                        <$implementor as $crate::macros::reflect::BaseType<$scalar>>::NAME,
                        "`: missing implementer reference in interface's `for` attribute.",
                    );
                    ::std::panic!("{}", MSG);
                }
            })*
        };
    };
}

/// Asserts that `impl = ...` attribute argument has all the types referencing
/// this GraphQL type in `#[graphql_interface(for = ...)]`.
///
/// Symmetrical to [`assert_implemented_for!`].
#[macro_export]
macro_rules! assert_interfaces_impls {
    ($scalar: ty, $interface: ty $(, $implementers: ty)* $(,)?) => {
        const _: () = {
            $({
                let is_present = $crate::macros::reflect::str_exists_in_arr(
                    <$interface as ::juniper::macros::reflect::BaseType<$scalar>>::NAME,
                    <$implementers as ::juniper::macros::reflect::Implements<$scalar>>::NAMES,
                );
                if !is_present {
                    const MSG: &str = $crate::reflect::const_concat!(
                        "Failed to implement interface `",
                        <$interface as $crate::macros::reflect::BaseType<$scalar>>::NAME,
                        "` on `",
                        <$implementers as $crate::macros::reflect::BaseType<$scalar>>::NAME,
                        "`: missing interface reference in implementer's `impl` attribute.",
                    );
                    ::std::panic!("{}", MSG);
                }
            })*
        };
    };
}

/// Asserts validness of [`Field`] [`Arguments`] and returned [`Type`].
///
/// This assertion is a combination of [`assert_subtype`] and
/// [`assert_field_args`].
///
/// See [spec][1] for more info.
///
/// [1]: https://spec.graphql.org/October2021#IsValidImplementation()
#[macro_export]
macro_rules! assert_field {
    (
        $base_ty: ty,
        $impl_ty: ty,
        $scalar: ty,
        $field_name: expr $(,)?
    ) => {
        $crate::assert_field_args!($base_ty, $impl_ty, $scalar, $field_name);
        $crate::assert_subtype!($base_ty, $impl_ty, $scalar, $field_name);
    };
}

/// Asserts validness of a [`Field`] return type.
///
/// See [spec][1] for more info.
///
/// [1]: https://spec.graphql.org/October2021#IsValidImplementationFieldType()
#[macro_export]
macro_rules! assert_subtype {
    (
        $base_ty: ty,
        $impl_ty: ty,
        $scalar: ty,
        $field_name: expr $(,)?
    ) => {
        const _: () = {
            const BASE_TY: $crate::macros::reflect::Type =
                <$base_ty as $crate::macros::reflect::BaseType<$scalar>>::NAME;
            const IMPL_TY: $crate::macros::reflect::Type =
                <$impl_ty as $crate::macros::reflect::BaseType<$scalar>>::NAME;
            const ERR_PREFIX: &str = $crate::reflect::const_concat!(
                "Failed to implement interface `",
                BASE_TY,
                "` on `",
                IMPL_TY,
                "`: ",
            );

            const FIELD_NAME: $crate::macros::reflect::Name =
                $field_name;

            const BASE_RETURN_WRAPPED_VAL: $crate::macros::reflect::WrappedValue =
                <$base_ty as $crate::macros::reflect::FieldMeta<
                    $scalar,
                    { $crate::checked_hash!(FIELD_NAME, $base_ty, $scalar, ERR_PREFIX) },
                >>::WRAPPED_VALUE;
            const IMPL_RETURN_WRAPPED_VAL: $crate::macros::reflect::WrappedValue =
                <$impl_ty as $crate::macros::reflect::FieldMeta<
                    $scalar,
                    { $crate::checked_hash!(FIELD_NAME, $impl_ty, $scalar, ERR_PREFIX) },
                >>::WRAPPED_VALUE;

            const BASE_RETURN_TY: $crate::macros::reflect::Type =
                <$base_ty as $crate::macros::reflect::FieldMeta<
                    $scalar,
                    { $crate::checked_hash!(FIELD_NAME, $base_ty, $scalar, ERR_PREFIX) },
                >>::TYPE;
            const IMPL_RETURN_TY: $crate::macros::reflect::Type =
                <$impl_ty as $crate::macros::reflect::FieldMeta<
                    $scalar,
                    { $crate::checked_hash!(FIELD_NAME, $impl_ty, $scalar, ERR_PREFIX) },
                >>::TYPE;

            const BASE_RETURN_SUB_TYPES: $crate::macros::reflect::Types =
                <$base_ty as $crate::macros::reflect::FieldMeta<
                    $scalar,
                    { $crate::checked_hash!(FIELD_NAME, $base_ty, $scalar, ERR_PREFIX) },
                >>::SUB_TYPES;

            let is_subtype = $crate::macros::reflect::str_exists_in_arr(IMPL_RETURN_TY, BASE_RETURN_SUB_TYPES)
                && $crate::macros::reflect::can_be_subtype(BASE_RETURN_WRAPPED_VAL, IMPL_RETURN_WRAPPED_VAL);
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

/// Asserts validness of the [`Field`]s arguments. See [spec][1] for more
/// info.
///
/// [1]: https://spec.graphql.org/October2021#sel-IAHZhCHCDEEFAAADHD8Cxob
#[macro_export]
macro_rules! assert_field_args {
    (
        $base_ty: ty,
        $impl_ty: ty,
        $scalar: ty,
        $field_name: expr $(,)?
    ) => {
        const _: () = {
            const BASE_NAME: &str = <$base_ty as $crate::macros::reflect::BaseType<$scalar>>::NAME;
            const IMPL_NAME: &str = <$impl_ty as $crate::macros::reflect::BaseType<$scalar>>::NAME;
            const ERR_PREFIX: &str = $crate::reflect::const_concat!(
                "Failed to implement interface `",
                BASE_NAME,
                "` on `",
                IMPL_NAME,
                "`: ",
            );

            const FIELD_NAME: &str = $field_name;

            const BASE_ARGS: ::juniper::macros::reflect::Arguments =
                <$base_ty as $crate::macros::reflect::FieldMeta<
                    $scalar,
                    { $crate::checked_hash!(FIELD_NAME, $base_ty, $scalar, ERR_PREFIX) },
                >>::ARGUMENTS;
            const IMPL_ARGS: ::juniper::macros::reflect::Arguments =
                <$impl_ty as $crate::macros::reflect::FieldMeta<
                    $scalar,
                    { $crate::checked_hash!(FIELD_NAME, $impl_ty, $scalar, ERR_PREFIX) },
                >>::ARGUMENTS;

            struct Error {
                cause: Cause,
                base: ::juniper::macros::reflect::Argument,
                implementation: ::juniper::macros::reflect::Argument,
            }

            enum Cause {
                RequiredField,
                AdditionalNonNullableField,
                TypeMismatch,
            }

            const fn unwrap_error(v: ::std::result::Result<(), Error>) -> Error {
                match v {
                    // Unfortunately we can't use `unreachable!()` here, as this
                    // branch will be executed either way.
                    Ok(()) => Error {
                        cause: Cause::RequiredField,
                        base: ("unreachable", "unreachable", 1),
                        implementation: ("unreachable", "unreachable", 1),
                    },
                    Err(err) => err,
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

                        if $crate::macros::reflect::str_eq(base_name, impl_name) {
                            if $crate::macros::reflect::str_eq(base_type, impl_type)
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
                        if $crate::macros::reflect::str_eq(base_name, impl_name) {
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

                const BASE_ARG_NAME: &str = ERROR.base.0;
                const IMPL_ARG_NAME: &str = ERROR.implementation.0;

                const BASE_TYPE_FORMATTED: &str =
                    $crate::reflect::format_type!(ERROR.base.1, ERROR.base.2);
                const IMPL_TYPE_FORMATTED: &str =
                    $crate::reflect::format_type!(ERROR.implementation.1, ERROR.implementation.2);

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
                            "` was expected, but not found."
                        )
                    }
                    Cause::AdditionalNonNullableField => {
                        $crate::reflect::const_concat!(
                            "Argument `",
                            IMPL_ARG_NAME,
                            "` of type `",
                            IMPL_TYPE_FORMATTED,
                            "` isn't present on the interface and so has to be nullable."
                        )
                    }
                };
                const ERROR_MSG: &str =
                    $crate::reflect::const_concat!(ERR_PREFIX, "Field `", FIELD_NAME, "`: ", MSG);
                ::std::panic!("{}", ERROR_MSG);
            }
        };
    };
}

/// Ensures that the given `$impl_ty` implements [`Field`] and returns a
/// [`fnv1a128`] hash for it, otherwise panics with understandable message.
#[macro_export]
macro_rules! checked_hash {
    ($field_name: expr, $impl_ty: ty, $scalar: ty $(, $prefix: expr)? $(,)?) => {{
        let exists = $crate::macros::reflect::str_exists_in_arr(
            $field_name,
            <$impl_ty as $crate::macros::reflect::Fields<$scalar>>::NAMES,
        );
        if exists {
            $crate::macros::reflect::fnv1a128(FIELD_NAME)
        } else {
            const MSG: &str = $crate::reflect::const_concat!(
                $($prefix,)?
                "Field `",
                $field_name,
                "` isn't implemented on `",
                <$impl_ty as $crate::macros::reflect::BaseType<$scalar>>::NAME,
                "`."
            );
            ::std::panic!("{}", MSG)
        }
    }};
}
