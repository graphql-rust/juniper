#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_impl_trait {
    (
        impl< < DefaultScalarValue > $(, $other: tt)* > $impl_trait:tt for $name:ty {
            $($body:tt)+
        }
    ) => {
        impl<$($other,)*> $crate::$impl_trait<$crate::DefaultScalarValue> for $name {
            $($body)*
        }
    };
    (
        impl< < DefaultScalarValue > $(, $other: tt)* > $impl_trait:tt for $name:ty
            where ( $($where:tt)* )
        {
            $($body:tt)+
        }
    ) => {
        impl<$($other,)*> $crate::$impl_trait<$crate::DefaultScalarValue> for $name
            where $($where)*
        {
            $($body)*
        }
    };

    (
        impl< <$generic:tt $(: $bound: tt)*> $(, $other: tt)* > $impl_trait:tt for $name:ty {
            $($body:tt)*
        }
    ) => {
       impl<$($other,)* $generic $(: $bound)*> $crate::$impl_trait<$generic> for $name
        where
            $generic: $crate::ScalarValue,
       {
           $($body)*
       }
    };
    (
        impl< <$generic:tt $(: $bound: tt)*> $(, $other: tt)* > $impl_trait:tt for $name:ty
            where ( $($where:tt)* )
        {
            $($body:tt)*
        }
    ) => {
       impl<$($other,)* $generic $(: $bound)*> $crate::$impl_trait<$generic> for $name
        where
            $($where)*
            $generic: $crate::ScalarValue,
       {
           $($body)*
       }
    };

    (
        impl<$scalar:ty $(, $other: tt )*> $impl_trait:tt for $name:ty {
            $($body:tt)*
        }
    ) => {
        impl<$($other, )*> $crate::$impl_trait<$scalar> for $name {
            $($body)*
        }
    };
    (
        impl<$scalar:ty $(, $other: tt )*> $impl_trait:tt for $name:ty
            where ( $($where:tt)* )
        {
            $($body:tt)*
        }
    ) => {
        impl<$($other, )*> $crate::$impl_trait<$scalar> for $name
            where $($where)*
        {
            $($body)*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_insert_generic {
    (<DefaultScalarValue>) => {
        $crate::DefaultScalarValue
    };
    (
        <$generic:tt $(: $bound: tt)*>
    ) => {
        $generic
    };
    (
        $scalar: ty
    ) => {
        $scalar
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_parse_object_header {
    (
        callback = $callback:ident,
        rest = <$($lifetime:tt),*> $name: ty $(: $ctxt: ty)* as $outname: tt
        where Scalar = <$generic:tt $(: $bound:tt)*> $(| &$mainself:ident |)* {
            $($items: tt)*
        }
    ) => {
        $crate::$callback!(
            @parse,
            meta = {
                lifetimes = [$($lifetime,)*],
                name = $name,
                $(ctx = $ctxt,)*
                $(main_self = $mainself,)*
                outname = {$outname},
                scalar = {<$generic $(: $bound)*>},
            },
            rest = $($items)*
        );
    };

    (
        callback = $callback:ident,
        rest = <$($lifetime:tt),*> $name: ty $(: $ctxt: ty)* as $outname: tt
        where Scalar = $scalar: ty $(| &$mainself:ident |)* {
            $($items: tt)*
        }
    ) => {
        $crate::$callback!(
            @parse,
            meta = {
                lifetimes = [$($lifetime,)*],
                name = $name,
                $(ctx = $ctxt,)*
                $(main_self = $mainself,)*
                outname = {$outname},
                scalar = {$scalar},
            },
            rest = $($items)*
        );
    };

    (
        callback = $callback: ident,
        rest = <$($lifetime:tt),*> $name: ty $(: $ctxt: ty)* as $outname: tt $(| &$mainself:ident |)* {
            $($items: tt)*
        }
    ) => {
        $crate::$callback!(
            @parse,
            meta = {
                lifetimes = [$($lifetime,)*],
                name = $name,
                $(ctx = $ctxt,)*
                $(main_self = $mainself,)*
                outname = {$outname},
                scalar = {<DefaultScalarValue>},
            },
            rest = $($items)*
        );
    };

    (
        callback = $callback: ident,
        rest = $name: ty $(: $ctxt: ty)* as $outname: tt
        where Scalar = <$generic:tt $(: $bound:tt)*> $(| &$mainself:ident |)* {
            $($items: tt)*
        }
    ) => {
        $crate::$callback!(
            @parse,
            meta = {
                lifetimes = [],
                name = $name,
                $(ctx = $ctxt,)*
                $(main_self = $mainself,)*
                outname = {$outname},
                scalar = {<$generic $(:$bound)*>},
            },
            rest = $($items)*
        );
    };

    (
        callback = $callback: ident,
        rest = $name: ty $(: $ctxt: ty)* as $outname: tt
        where Scalar = $scalar: ty $(| &$mainself:ident |)* {
            $($items: tt)*
        }
    ) => {
        $crate::$callback!(
            @parse,
            meta = {
                lifetimes = [],
                name = $name,
                $(ctx = $ctxt,)*
                $(main_self = $mainself,)*
                outname = {$outname},
                scalar = {$scalar},
            },
            rest = $($items)*
        );
    };


    (
        callback = $callback: ident,
        rest = $name: ty $(: $ctxt: ty)* as $outname: tt $(| &$mainself:ident |)* {
            $($items: tt)*
        }
    ) => {
        $crate::$callback!(
            @parse,
            meta = {
                lifetimes = [],
                name = $name,
                $(ctx = $ctxt,)*
                $(main_self = $mainself,)*
                outname = {$outname},
                scalar = {<DefaultScalarValue>},
            },
            rest = $($items)*
        );
    };

    (
        callback = $callback: ident,
        rest = <$($lifetime:tt),*> $name: ty $(: $ctxt: ty)*
        where Scalar = <$generic:tt $(: $bound:tt)*> $(| &$mainself:ident |)* {
            $($items: tt)*
        }
    ) => {
        $crate::$callback!(
            @parse,
            meta = {
                lifetimes = [$($lifetime,)*],
                name = $name,
                $(ctx = $ctxt,)*
                $(main_self = $mainself,)*
                outname = {stringify!($name)},
                scalar = {<$generic $(:$bounds)*>},
            },
            rest = $($items)*
        );
    };

    (
        callback = $callback: ident,
        rest = <$($lifetime:tt),*> $name: ty $(: $ctxt: ty)*
        where Scalar = $scalar: ty $(| &$mainself:ident |)* {
            $($items: tt)*
        }
    ) => {
        $crate::$callback!(
            @parse,
            meta = {
                lifetimes = [$($lifetime,)*],
                name = $name,
                $(ctx = $ctxt,)*
                $(main_self = $mainself,)*
                outname = {stringify!($name)},
                scalar = {$scalar},
            },
            rest = $($items)*
        );
    };

    (
        callback = $callback: ident,
        rest = <$($lifetime:tt),*> $name: ty $(: $ctxt: ty)* $(| &$mainself:ident |)* {
            $($items: tt)*
        }
    ) => {
        $crate::$callback!(
            @parse,
            meta = {
                lifetimes = [$($lifetime,)*],
                name = $name,
                $(ctx = $ctxt,)*
                $(main_self = $mainself,)*
                outname = {stringify!($name)},
                scalar = {<DefaultScalarValue>},
            },
            rest = $($items)*
        );
    };


    (
        callback = $callback: ident,
        rest = $name: ty $(: $ctxt: ty)*
        where Scalar = <$generic:tt $(: $bound:tt)*> $(| &$mainself:ident |)*
        {
            $($items: tt)*
        }
    ) => {
        $crate::$callback!(
            @parse,
            meta = {
                lifetimes = [],
                name = $name,
                $(ctx = $ctxt,)*
                $(main_self = $mainself,)*
                outname = {stringify!($name)},
                scalar = {<$generic $(: $bound)*>},
            },
            rest = $($items)*
        );
    };

    (
        callback = $callback: ident,
        rest = $name: ty $(: $ctxt: ty)* where Scalar = $scalar: ty $(| &$mainself:ident |)* {
            $($items: tt)*
        }
    ) => {
        $crate::$callback!(
            @parse,
            meta = {
                lifetimes = [],
                name = $name,
                $(ctx = $ctxt,)*
                $(main_self = $mainself,)*
                outname = {stringify!($name)},
                scalar = {$scalar},
            },
            rest = $($items)*
        );
    };

    (
        callback = $callback: ident,
        rest = $name: ty $(: $ctxt: ty)* $(| &$mainself:ident |)* {
            $($items: tt)*
        }
    ) => {
        $crate::$callback!(
            @parse,
            meta = {
                lifetimes = [],
                name = $name,
                $(ctx = $ctxt,)*
                $(main_self = $mainself,)*
                outname = {stringify!($name)},
                scalar = {<DefaultScalarValue>},
            },
            rest = $($items)*
        );
    };
    (
        callback = $callback: ident,
        rest = $($rest:tt)*
    ) => {
        compile_error!("Invalid syntax");
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_parse_field_list {
    (
        success_callback = $success_callback: ident,
        additional_parser = {$($additional:tt)*},
        meta = {$($meta:tt)*},
        items = [$({$($items: tt)*},)*],
        rest =
    ) => {
        $crate::$success_callback!(
            @generate,
            meta = {$($meta)*},
            items = [$({$($items)*},)*],
        );
    };

    (
        success_callback = $success_callback: ident,
        additional_parser = {$($additional:tt)*},
        meta = {$($meta:tt)*},
        items = [$({$($items: tt)*},)*],
        rest = , $($rest: tt)*
    ) => {
        $crate::__juniper_parse_field_list!(
            success_callback = $success_callback,
            additional_parser = {$($additional)*},
            meta = {$($meta)*},
            items = [$({$($items)*},)*],
            rest = $($rest)*
        );
    };


    (
        @parse_description,
        success_callback = $success_callback: ident,
        additional_parser = {$($additional:tt)*},
        meta = {
            $(lifetimes = [$($lifetime:tt,)*],)*
            $(name = $name:ty,)*
            $(ctx = $ctxt: ty,)*
            $(main_self = $mainself: ident,)*
            $(outname = {$($outname:tt)*},)*
            $(scalar = {$($scalar:tt)*},)*
            $(description = $_desciption: tt,)*
            $(additional = {$($other: tt)*},)*
        },
        items = [$({$($items: tt)*},)*],
        rest = $desc: tt  $($rest:tt)*
    ) => {
        $crate::__juniper_parse_field_list!(
            success_callback = $success_callback,
            additional_parser = {$($additional)*},
            meta = {
                $(lifetimes = [$($lifetime,)*],)*
                $(name = $name,)*
                $(ctx = $ctxt,)*
                $(main_self = $mainself,)*
                $(outname = {$($outname)*},)*
                $(scalar = {$($scalar)*},)*
                description = $desc,
                $(additional = {$($other)*},)*

            },
            items = [$({$($items)*},)*],
            rest = $($rest)*
        );
    };
    (
        success_callback = $success_callback: ident,
        additional_parser = {$($additional:tt)*},
        meta = { $($meta:tt)*},
        items = [$({$($items: tt)*},)*],
        rest = description:  $($rest:tt)*
    ) => {
        $crate::__juniper_parse_field_list!(
            @parse_description,
            success_callback = $success_callback,
            additional_parser = {$($additional)*},
            meta = {$($meta)*},
            items = [$({$($items)*},)*],
            rest = $($rest)*
        );
    };

    (
        success_callback = $success_callback: ident,
        additional_parser = {$($additional:tt)*},
        meta = {$($meta:tt)*},
        items = [$({$($items: tt)*},)*],
        rest = $(#[doc = $desc: tt])*
        #[deprecated $(( $(since = $since: tt,)* note = $reason: tt ))* ]
        field $name: ident (
            $(&$executor: tt)* $(,)*
            $($(#[doc = $arg_desc: expr])* $arg_name:ident $(= $arg_default: tt)* : $arg_ty: ty),* $(,)*
        ) -> $return_ty: ty $body: block
            $($rest:tt)*
    ) => {
        $crate::__juniper_parse_field_list!(
            success_callback = $success_callback,
            additional_parser = {$($additional)*},
            meta = {$($meta)*},
            items = [$({$($items)*},)* {
                name = $name,
                body = $body,
                return_ty = $return_ty,
                args = [
                    $({
                        arg_name = $arg_name,
                        arg_ty = $arg_ty,
                        $(arg_default = $arg_default,)*
                        $(arg_docstring = $arg_desc,)*
                    },)*
                ],
                $(docstring = $desc,)*
                deprecated = None$(.unwrap_or_else(|| Some($reason)))*,
                $(executor_var = $executor,)*
            },],
            rest = $($rest)*
        );
    };
    (
        success_callback = $success_callback: ident,
        additional_parser = {$($additional:tt)*},
        meta = {$($meta:tt)*},
        items = [$({$($items: tt)*},)*],
        rest = $(#[doc = $desc: tt])*
        field $name: ident (
            $(&$executor: ident)* $(,)*
            $($(#[doc = $arg_desc: expr])* $arg_name:ident $(= $arg_default: tt)* : $arg_ty: ty),* $(,)*
        ) -> $return_ty: ty $body: block
            $($rest:tt)*
    ) => {
        $crate::__juniper_parse_field_list!(
            success_callback = $success_callback,
            additional_parser = {$($additional)*},
            meta = {$($meta)*},
            items = [$({$($items)*},)* {
                name = $name,
                body = $body,
                return_ty = $return_ty,
                args = [
                    $({
                        arg_name = $arg_name,
                        arg_ty = $arg_ty,
                        $(arg_default = $arg_default,)*
                        $(arg_docstring = $arg_desc,)*
                    },)*
                ],
                $(docstring = $desc,)*
                $(executor_var = $executor,)*
            },],
            rest = $($rest)*
        );
    };
    (
        success_callback = $success_callback: ident,
        additional_parser = {$($additional:tt)*},
        meta = {$($meta:tt)*},
        items = [$({$($items: tt)*},)*],
        rest = field deprecated $reason:tt $name: ident (
            $(&$executor: tt)* $(,)*
            $($arg_name:ident $(= $arg_default: tt)* : $arg_ty: ty $(as $arg_desc: expr)*),* $(,)*
        ) -> $return_ty: ty $(as $desc: tt)* $body: block
            $($rest:tt)*
    ) => {
        $crate::__juniper_parse_field_list!(
            success_callback = $success_callback,
            additional_parser = {$($additional)*},
            meta = {$($meta)*},
            items = [$({$($items)*},)* {
                name = $name,
                body = $body,
                return_ty = $return_ty,
                args = [
                    $({
                        arg_name = $arg_name,
                        arg_ty = $arg_ty,
                        $(arg_default = $arg_default,)*
                        $(arg_description = $arg_desc,)*
                    },)*
                ],
                $(decs = $desc,)*
                deprecated = Some($reason),
                $(executor_var = $executor,)*
            },],
            rest = $($rest)*
        );
    };
    (
        success_callback = $success_callback: ident,
        additional_parser = {$($additional:tt)*},
        meta = {$($meta:tt)*},
        items = [$({$($items: tt)*},)*],
        rest = field $name: ident (
            $(&$executor: ident)* $(,)*
            $($arg_name:ident $(= $arg_default: tt)* : $arg_ty: ty $(as $arg_desc: expr)*),* $(,)*
        ) -> $return_ty: ty $(as $desc: tt)* $body: block
            $($rest:tt)*
    ) => {
        $crate::__juniper_parse_field_list!(
            success_callback = $success_callback,
            additional_parser = {$($additional)*},
            meta = {$($meta)*},
            items = [$({$($items)*},)* {
                name = $name,
                body = $body,
                return_ty = $return_ty,
                args = [
                    $({
                        arg_name = $arg_name,
                        arg_ty = $arg_ty,
                        $(arg_default = $arg_default,)*
                        $(arg_description = $arg_desc,)*
                    },)*
                ],
                $(decs = $desc,)*
                $(executor_var = $executor,)*
            },],
            rest = $($rest)*
        );
    };

    (
        success_callback = $success_callback: ident,
        additional_parser = {
            callback = $callback: ident,
            header = {$($header:tt)*},
        },
        meta = {$($meta:tt)*},
        items = [$({$($items: tt)*},)*],
        rest = $($rest:tt)*
    ) => {
        $crate::$callback!(
            $($header)*
            success_callback = $success_callback,
            additional_parser = {
                callback = $callback,
                header = {$($header)*},
            },
            meta = {$($meta)*},
            items = [$({$($items)*},)*],
            rest = $($rest)*
        );
    }

}

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_parse_instance_resolver {
    (
        success_callback = $success_callback: ident,
        additional_parser = {$($additional:tt)*},
        meta = {
            lifetimes = [$($lifetime:tt,)*],
            name = $name:ty,
            ctx = $ctxt:ty,
            main_self = $mainself:ident,
            outname = {$($outname:tt)*},
            scalar = {$($scalar:tt)*},
            $(description = $desciption:tt,)*
                $(additional = {
                    $(resolver = {$($ignored_resolver:tt)*},)*
                },)*

        },
        items = [$({$($items: tt)*},)*],
        rest = instance_resolvers: |&$context: ident| {
            $( $srctype:ty => $resolver:expr ),* $(,)*
        } $($rest:tt)*
    ) => {
        $crate::__juniper_parse_field_list!(
            success_callback = $success_callback,
            additional_parser = {$($additional)*},
            meta = {
                lifetimes = [$($lifetime,)*],
                name = $name,
                ctx = $ctxt,
                main_self = $mainself,
                outname = {$($outname)*},
                scalar = {$($scalar)*},
                $(description = $desciption,)*
                additional = {
                    resolver = {
                        context = $context,
                        items = [
                            $({
                                src = $srctype,
                                resolver = $resolver,
                            },)*
                        ],
                    },
                },
            },
            items = [$({$($items)*},)*],
            rest = $($rest)*
        );
    };

    (
        success_callback = $success_callback: ident,
        additional_parser = {$($additional:tt)*},
        meta = {
            lifetimes = [$($lifetime:tt,)*],
            name = $name:ty,
            ctx = $ctxt:ty,
            main_self = $mainself:ident,
            outname = {$($outname:tt)*},
            scalar = {$($scalar:tt)*},
            $(description = $desciption:tt,)*
            $(additional = {
                $(resolver = {$($ignored_resolver:tt)*},)*
            },)*

        },
        items = [$({$($items: tt)*},)*],
        rest = instance_resolvers: |$(&)* _| {$( $srctype:ty => $resolver:expr ),* $(,)*} $($rest:tt)*
    ) => {
        $crate::__juniper_parse_field_list!(
            success_callback = $success_callback,
            additional_parser = {$($additional)*},
            meta = {
                lifetimes = [$($lifetime,)*],
                name = $name,
                ctx = $ctxt,
                main_self = $mainself,
                outname = {$($outname)*},
                scalar = {$($scalar)*},
                $(description = $desciption,)*
                additional = {
                    resolver = {
                        items = [
                            $({
                                src = $srctype,
                                resolver = $resolver,
                            },)*
                        ],
                    },
                },
            },
            items = [$({$($items)*},)*],
            rest = $($rest)*
        );
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __juniper_create_arg {
    (
        registry = $reg: ident,
        info = $info: ident,
        arg_ty = $arg_ty: ty,
        arg_name = $arg_name: ident,
        $(description = $arg_description: expr,)*
        $(docstring = $arg_docstring: expr,)*
    ) => {
        $reg.arg::<$arg_ty>(
            &$crate::to_camel_case(stringify!($arg_name)),
            $info,
        )
        $(.description($arg_description))*
        .push_docstring(&[$($arg_docstring,)*])
    };

    (
        registry = $reg: ident,
        info = $info: ident,
        arg_ty = $arg_ty: ty,
        arg_name = $arg_name: ident,
        default = $arg_default: expr,
        $(description = $arg_description: expr,)*
        $(docstring = $arg_docstring: expr,)*
    ) => {
        $reg.arg_with_default::<$arg_ty>(
            &$crate::to_camel_case(stringify!($arg_name)),
            &($arg_default),
            $info,
        )
        $(.description($arg_description))*
        .push_docstring(&[$($arg_docstring,)*])
    };
}
