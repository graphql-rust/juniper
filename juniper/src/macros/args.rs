#[doc(hidden)]
#[macro_export(local_inner_macros)]
macro_rules! __graphql__args {
    // Internal type conversion
    ( @as_expr, $e:expr) => { $e };
    ( @as_pattern, $p:pat) => { $p };

    ( @assign_arg_vars, $args:ident, $executorvar:ident, , $($rest:tt)* ) => {
        __graphql__args!(@assign_arg_vars, $args, $executorvar, $($rest)*);
    };

    ( @assign_arg_vars, $args:ident, $executorvar:ident, ) => {
        ();
    };

    (
        @assign_arg_vars,
        $args:ident, $executorvar:ident, &$exec:ident $($rest:tt)*
    ) => {
        let __graphql__args!(@as_pattern, $exec) = &$executorvar;
        __graphql__args!(@assign_arg_vars, $args, $executorvar, $($rest)*);
    };

    (
        @assign_arg_vars,
        $args:ident, $executorvar:ident,
        $name:ident $(= $default:tt)* : $ty:ty $(as $desc:tt)*, $($rest:tt)*
    ) => {
        let $name: $ty = $args
            .get(&$crate::to_camel_case(__graphql__stringify!($name)))
            .expect("Argument missing - validation must have failed");
        __graphql__args!(@assign_arg_vars, $args, $executorvar, $($rest)*);
    };

    (
        @assign_arg_vars,
        $args:ident, $executorvar:ident,
        $name:ident  $(= $default:tt)* : $ty:ty $(as $desc:expr)*
    ) => {
        let $name: $ty = $args
            .get(&$crate::to_camel_case(__graphql__stringify!($name)))
            .expect("Argument missing - validation must have failed");
    };

    ( @apply_args, $reg:expr, $base:expr, $info:expr, ( ) ) => {
        $base
    };

    (
        @apply_args,
        $reg:expr, $base:expr, $info:expr, ( , $( $rest:tt )* )
    ) => {
        __graphql__args!(
            @apply_args,
            $reg,
            $base,
            $info,
            ( $($rest)* ))
    };

    (
        @apply_args,
        $reg:expr, $base:expr, $info:expr, ( &executor $( $rest:tt )* )
    ) => {
        __graphql__args!(
            @apply_args,
            $reg,
            $base,
            $info,
            ( $($rest)* ))
    };

    (
        @apply_args,
        $reg:expr, $base:expr, $info:expr, ( $name:ident = $default:tt : $t:ty )
    ) => {
        $base.argument($reg.arg_with_default::<$t>(
            &$crate::to_camel_case(__graphql__stringify!($name)),
            &__graphql__args!(@as_expr, $default), $info))
    };

    (
        @apply_args,
        $reg:expr, $base:expr, $info:expr, ( $name:ident = $default:tt : $t:ty , $( $rest:tt )* )
    ) => {
        __graphql__args!(
            @apply_args,
            $reg,
            $base.argument($reg.arg_with_default::<$t>(
                &$crate::to_camel_case(__graphql__stringify!($name)),
                &__graphql__args!(@as_expr, $default), $info)),
            $info,
            ( $($rest)* ))
    };

    (
        @apply_args,
        $reg:expr, $base:expr, $info:expr,
        ( $name:ident = $default:tt : $t:ty as $desc:tt $( $rest:tt )* )
    ) => {
        __graphql__args!(
            @apply_args,
            $reg,
            $base.argument($reg.arg_with_default::<$t>(
                &$crate::to_camel_case(__graphql__stringify!($name)),
                &__graphql__args!(@as_expr, $default), $info)
                .description($desc)),
            $info,
            ( $($rest)* ))
    };

    (
        @apply_args,
        $reg:expr, $base:expr, $info:expr, ( $name:ident : $t:ty )
    ) => {
        $base.argument($reg.arg::<$t>(
            &$crate::to_camel_case(__graphql__stringify!($name)), $info))
    };

    (
        @apply_args,
        $reg:expr, $base:expr, $info:expr, ( $name:ident : $t:ty , $( $rest:tt )* )
    ) => {
        __graphql__args!(
            @apply_args,
            $reg,
            $base.argument($reg.arg::<$t>(
                &$crate::to_camel_case(__graphql__stringify!($name)), $info)),
            $info,
            ( $($rest)* ))
    };

    (
        @apply_args,
        $reg:expr, $base:expr, $info:expr, ( $name:ident : $t:ty as $desc:tt $( $rest:tt )* )
    ) => {
        __graphql__args!(
            @apply_args,
            $reg,
            $base.argument(
                $reg.arg::<$t>(
                    &$crate::to_camel_case(__graphql__stringify!($name)), $info)
                .description($desc)),
            $info,
            ( $($rest)* ))
    };
}
