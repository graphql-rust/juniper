/// Expose GraphQL scalars
///
/// The GraphQL language defines a number of built-in scalars: strings, numbers, and
/// booleans. This macro can be used either to define new types of scalars (e.g.
/// timestamps), or expose other types as one of the built-in scalars (e.g. bigints
/// as numbers or strings).
///
/// Since the preferred transport protocol for GraphQL responses is JSON, most
/// custom scalars will be transferred as strings. You therefore need to ensure that
/// the client library you are sending data to can parse the custom value into a
/// datatype appropriate for that platform.
///
/// By default the trait is implemented in terms of the default scalar value
/// representation provided by juniper. If that does not fit your needs it is
/// possible to use the same syntax as on `graphql_object!` to specify a custom
/// representation.
///
/// ```rust
/// # extern crate juniper;
/// # use juniper::{Value, FieldResult, ParseScalarValue, ParseScalarResult};
/// struct UserID(String);
///
/// juniper::graphql_scalar!(UserID {
///     description: "An opaque identifier, represented as a string"
///
///     resolve(&self) -> Value {
///         Value::string(&self.0)
///     }
///
///     from_input_value(v: &InputValue) -> Option<UserID> {
///     v.as_string_value().map(|s| UserID(s.to_owned()))
///     }
///
///     from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a> {
///         <String as ParseScalarValue>::from_str(value)
///     }
/// });
///
/// # fn main() { }
/// ```
///
/// In addition to implementing `GraphQLType` for the type in question,
/// `FromInputValue` and `ToInputValue` is also implemented. This makes the type
/// usable as arguments and default values.
#[macro_export]
macro_rules! graphql_scalar {
    ( @as_expr $e:expr) => { $e };

    (
        @generate,
        meta = {
            name = $name:ty,
            outname = {$($outname:tt)+},
            scalar = {$($scalar:tt)+},
            $(description = $descr:tt,)*
        },
        resolve = {
            self_var = $resolve_self_var:ident,
            body = $resolve_body: block,
            return_type = $resolve_retun_type: ty,
        },
        from_input_value = {
            arg = $from_input_value_arg: ident,
            result = $from_input_value_result: ty,
            body = $from_input_value_body: block,
        },
        from_str = {
            value_arg = $from_str_arg: ident,
            result = $from_str_result: ty,
            body = $from_str_body: block,
            lifetime = $from_str_lt: tt,
        },

    ) => {
        $crate::__juniper_impl_trait!(
            impl <$($scalar)+> GraphQLType for $name {
                type Context = ();
                type TypeInfo = ();

                fn name(_: &Self::TypeInfo) -> Option<&str> {
                    Some($crate::graphql_scalar!(@as_expr $($outname)+))
                }

                fn meta<'r>(
                    info: &Self::TypeInfo,
                    registry: &mut $crate::Registry<'r, $crate::__juniper_insert_generic!($($scalar)+)>
                ) -> $crate::meta::MetaType<'r, $crate::__juniper_insert_generic!($($scalar)+)>
                where
                    $crate::__juniper_insert_generic!($($scalar)+): 'r
                {
                    let meta = registry.build_scalar_type::<Self>(info);
                    $(
                        let meta = meta.description($descr);
                    )*
                    meta.into_meta()
                }

                fn resolve(
                    &$resolve_self_var,
                    _: &(),
                    _: Option<&[$crate::Selection<$crate::__juniper_insert_generic!($($scalar)+)>]>,
                    _: &$crate::Executor<
                        Self::Context,
                        $crate::__juniper_insert_generic!($($scalar)+)
                    >) -> $crate::ExecutionResult<$crate::__juniper_insert_generic!($($scalar)+)> {
                    Ok($resolve_body)
                }
            }
        );

        $crate::__juniper_impl_trait!(
            impl <$($scalar)+> GraphQLTypeAsync for $name
                where (
                    $crate::__juniper_insert_generic!($($scalar)+): Send + Sync,
                    Self: $crate::GraphQLType<$crate::__juniper_insert_generic!($($scalar)+)> + Send + Sync,
                    Self::Context: Send + Sync,
                    Self::TypeInfo: Send + Sync,
                )
            {

                fn resolve_async<'a>(
                    &'a self,
                    info: &'a Self::TypeInfo,
                    selection_set: Option<&'a [$crate::Selection<$crate::__juniper_insert_generic!($($scalar)+)>]>,
                    executor: &'a $crate::Executor<Self::Context, $crate::__juniper_insert_generic!($($scalar)+)>,
                ) -> $crate::BoxFuture<'a, $crate::ExecutionResult<$crate::__juniper_insert_generic!($($scalar)+)>> {
                    use $crate::GraphQLType;
                    use futures::future;
                    let v = self.resolve(info, selection_set, executor);
                    Box::pin(future::ready(v))
                }
            }
        );

        $crate::__juniper_impl_trait!(
            impl<$($scalar)+> ToInputValue for $name {
                fn to_input_value(&$resolve_self_var) -> $crate::InputValue<$crate::__juniper_insert_generic!($($scalar)+)> {
                    let v = $resolve_body;
                    $crate::ToInputValue::to_input_value(&v)
                }
            }
        );

        $crate::__juniper_impl_trait!(
            impl<$($scalar)+> FromInputValue for $name {
                fn from_input_value(
                    $from_input_value_arg: &$crate::InputValue<$crate::__juniper_insert_generic!($($scalar)+)>
                ) -> $from_input_value_result {
                    $from_input_value_body
                }
            }
        );

        $crate::__juniper_impl_trait!(
            impl<$($scalar)+> ParseScalarValue for $name {
                fn from_str<$from_str_lt>($from_str_arg: $crate::parser::ScalarToken<$from_str_lt>) -> $from_str_result {
                    $from_str_body
                }
            }
        );
    };

    // No more items to parse
    (
        @parse_functions,
        meta = {
            name = $name:ty,
            outname = {$($outname:tt)+},
            scalar = {$($scalar:tt)+},
            $(description = $descr:tt,)*
        },
        resolve = {$($resolve_body:tt)+},
        from_input_value = {$($from_input_value_body:tt)+},
        from_str = {$($from_str_body:tt)+},
        rest =
    ) => {
        $crate::graphql_scalar!(
            @generate,
            meta = {
                name = $name,
                outname = {$($outname)+},
                scalar = {$($scalar)+},
                $(description = $descr,)*
            },
            resolve = {$($resolve_body)+},
            from_input_value = {$($from_input_value_body)+},
            from_str = {$($from_str_body)+},
        );
    };

    (
        @parse_functions,
        meta = {
            name = $name:ty,
            outname = {$($outname:tt)+},
            scalar = {$($scalar:tt)+},
            $(description = $descr:tt,)*
        },
        $(from_input_value = {$($from_input_value_body:tt)+})*,
        $(from_str = {$($from_str_body:tt)+})*,
        rest =
    ) => {
        compile_error!("Missing resolve function");
    };

    (
        @parse_functions,
        meta = {
            name = $name:ty,
            outname = {$($outname:tt)+},
            scalar = {$($scalar:tt)+},
            $(description = $descr:tt,)*
        },
        resolve = {$($resolve_body:tt)+},
        $(from_str = {$($from_str_body:tt)+})*,
        rest =
    ) => {
        compile_error!("Missing from_input_value function");
    };

    (
        @parse_functions,
        meta = {
            name = $name:ty,
            outname = {$($outname:tt)+},
            scalar = {$($scalar:tt)+},
            $(description = $descr:tt,)*
        },
        resolve = {$($resolve_body:tt)+},
        from_input_value = {$($from_input_value_body:tt)+},
        rest =
    ) =>{
        compile_error!("Missing from_str function");
    };


    // resolve(&self) -> Value { ... }
    (
        @parse_functions,
        meta = {$($meta:tt)*},
        $(resolve = {$($resolve_body:tt)+},)*
        $(from_input_value = {$($from_input_value_body:tt)+},)*
        $(from_str = {$($from_str_body:tt)+},)*
        rest = resolve(&$selfvar:ident) -> $return_ty:ty $body:block $($rest:tt)*
    ) => {
        $crate::graphql_scalar!(
            @parse_functions,
            meta = {$($meta)*},
            resolve = {
                self_var = $selfvar,
                body = $body,
                return_type = $return_ty,
            },
            $(from_input_value = {$($from_input_value_body)+},)*
            $(from_str = {$($from_str_body)+},)*
            rest = $($rest)*
        );
    };

    // from_input_value(arg: &InputValue) -> ... { ... }
    (
        @parse_functions,
        meta = { $($meta:tt)* },
        $(resolve = {$($resolve_body:tt)+})*,
        $(from_input_value = {$($from_input_value_body:tt)+},)*
        $(from_str = {$($from_str_body:tt)+},)*
        rest = from_input_value($arg:ident: &InputValue) -> $result:ty $body:block $($rest:tt)*
    ) => {
        $crate::graphql_scalar!(
            @parse_functions,
            meta = { $($meta)* },
            $(resolve = {$($resolve_body)+},)*
            from_input_value = {
                arg = $arg,
                result = $result,
                body = $body,
            },
            $(from_str = {$($from_str_body)+},)*
            rest = $($rest)*
        );
    };

    // from_str(value: &str) -> Result<S, ParseError>
    (
        @parse_functions,
        meta = { $($meta:tt)* },
        $(resolve = {$($resolve_body:tt)+},)*
        $(from_input_value = {$($from_input_value_body:tt)+},)*
        $(from_str = {$($from_str_body:tt)+},)*
        rest = from_str<$from_str_lt: tt>($value_arg:ident: ScalarToken<$ignored_lt2: tt>) -> $result:ty $body:block $($rest:tt)*
    ) => {
        $crate::graphql_scalar!(
            @parse_functions,
            meta = { $($meta)* },
            $(resolve = {$($resolve_body)+},)*
            $(from_input_value = {$($from_input_value_body)+},)*
            from_str = {
                value_arg = $value_arg,
                result = $result,
                body = $body,
                lifetime = $from_str_lt,
            },
            rest = $($rest)*
        );
    };

    // description: <description>
    (
        @parse_functions,
        meta = {
            name = $name:ty,
            outname = {$($outname:tt)+},
            scalar = {$($scalar:tt)+},
        },
        $(resolve = {$($resolve_body:tt)+},)*
        $(from_input_value = {$($from_input_value_body:tt)+},)*
        $(from_str = {$($from_str_body:tt)+},)*
        rest = description: $descr:tt $($rest:tt)*
    ) => {
        $crate::graphql_scalar!(
            @parse_functions,
            meta = {
                name = $name,
                outname = {$($outname)+},
                scalar = {$($scalar)+},
                description = $descr,
            },
            $(resolve = {$($resolve_body)+},)*
            $(from_input_value = {$($from_input_value_body)+},)*
            $(from_str = {$($from_str_body)+},)*
            rest = $($rest)*
        );
    };

    (
        @parse,
        meta = {
            lifetimes = [],
            name = $name: ty,
            outname = {$($outname:tt)*},
            scalar = {$($scalar:tt)*},
        },
        rest = $($rest:tt)*
    ) => {
         $crate::graphql_scalar!(
            @parse_functions,
            meta = {
                name = $name,
                outname = {$($outname)*},
                scalar = {$($scalar)*},
            },
            rest = $($rest)*
        );
    };

    (@$($stuff:tt)*) => {
        compile_error!("Invalid syntax for `graphql_scalar!`");
    };

    ($($rest:tt)*) => {
        $crate::__juniper_parse_object_header!(
            callback = graphql_scalar,
            rest = $($rest)*
        );
    }
}
