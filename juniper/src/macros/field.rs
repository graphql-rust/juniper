#[doc(hidden)]
#[macro_export]
macro_rules! __graphql__build_field_matches {
    // field deprecated <reason> <name>(...) -> <type> as <description> { ... }
    (
        $resolveargs:tt,
        ( $( $acc:tt )* ),
        field deprecated $_reason:tt $name:ident $args:tt -> $t:ty as $desc:tt $body:block $( $rest:tt )*
    ) => {
        __graphql__build_field_matches!(
            $resolveargs,
            (($name; $args; $t; $body) $( $acc )*),
            $( $rest )*);
    };

    // field deprecated <reason> <name>(...) -> <type> { ... }
    (
        $resolveargs:tt,
        ( $( $acc:tt )* ),
        field deprecated $_reason:tt $name:ident $args:tt -> $t:ty $body:block $( $rest:tt )*
    ) => {
        __graphql__build_field_matches!(
            $resolveargs,
            (($name; $args; $t; $body) $( $acc )*),
            $( $rest )*);
    };

    // field <name>(...) -> <type> as <description> { ... }
    (
        $resolveargs:tt,
        ( $( $acc:tt )* ), field $name:ident $args:tt -> $t:ty as $desc:tt $body:block $( $rest:tt )*
    ) => {
        __graphql__build_field_matches!(
            $resolveargs,
            (($name; $args; $t; $body) $( $acc )*),
            $( $rest )*);
    };

    // field <name>(...) -> <type> { ... }
    (
        $resolveargs:tt,
        ( $( $acc:tt )* ), field $name:ident $args:tt -> $t:ty $body:block $( $rest:tt )*
    ) => {
        __graphql__build_field_matches!(
            $resolveargs,
            (($name; $args; $t; $body) $( $acc )*),
            $( $rest )*);
    };

    ( $resolveargs:tt, $acc:tt, description : $value:tt $( $rest:tt )*) => {
        __graphql__build_field_matches!($resolveargs, $acc, $( $rest )*);
    };

    ( $resolveargs:tt, $acc:tt, interfaces : $value:tt $( $rest:tt )*) => {
        __graphql__build_field_matches!($resolveargs, $acc, $( $rest )*);
    };

    ( $resolveargs:tt, $acc:tt, instance_resolvers : | $execvar:pat | $resolvers:tt $( $rest:tt )*) => {
        __graphql__build_field_matches!($resolveargs, $acc, $( $rest )*);
    };

    ( $resolveargs:tt, $acc:tt, , $( $rest:tt )*) => {
        __graphql__build_field_matches!($resolveargs, $acc, $( $rest )*);
    };

    (
        ($outname:tt, $selfvar:ident, $fieldvar:ident, $argsvar:ident, $executorvar:ident),
        ( $( ( $name:ident; ( $($args:tt)* ); $t:ty; $body:block ) )* ),
    ) => {
        $(
            if $fieldvar == &$crate::to_camel_case(stringify!($name)) {
                let result: $t = (||{
                    __graphql__args!(
                        @assign_arg_vars,
                        $argsvar, $executorvar, $($args)*
                    );
                    $body
                })();

                return ($crate::IntoResolvable::into(result, $executorvar.context())).and_then(
                    |res| match res {
                        Some((ctx, r)) => $executorvar.replaced_context(ctx).resolve_with_ctx(&(), &r),
                        None => Ok($crate::Value::null()),
                    })
            }
        )*
        panic!("Field {} not found on type {}", $fieldvar, $outname);
    };
}
