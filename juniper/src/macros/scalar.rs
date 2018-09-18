/**
Expose GraphQL scalars

The GraphQL language defines a number of built-in scalars: strings, numbers, and
booleans. This macro can be used either to define new types of scalars (e.g.
timestamps), or expose other types as one of the built-in scalars (e.g. bigints
as numbers or strings).

Since the preferred transport protocol for GraphQL responses is JSON, most
custom scalars will be transferred as strings. You therefore need to ensure that
the client library you are sending data to can parse the custom value into a
datatype appropriate for that platform.

```rust
# #[macro_use] extern crate juniper;
# use juniper::{Value, FieldResult, ParseScalarValue};
# use juniper::parser::ParseError;
struct UserID(String);

graphql_scalar!(UserID where Scalar = <S> {
    description: "An opaque identifier, represented as a string"

    resolve(&self) -> Value {
        Value::string(&self.0)
    }

    from_input_value(v: &InputValue) -> Option<UserID> {
        v.as_string_value().map(|s| UserID(s.to_owned()))
    }

    from_str<'a>(value: ScalarToken<'a>) -> Result<S, ParseError<'a>> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
});

# fn main() { }
```

In addition to implementing `GraphQLType` for the type in question,
`FromInputValue` and `ToInputValue` is also implemented. This makes the type
usable as arguments and default values.

*/
#[macro_export(local_inner_macros)]
macro_rules! graphql_scalar {
    ( @as_expr $e:expr) => { $e };
    (
        @insert_generic
        <$generic:tt>
    ) => {
        $generic
    };
    (
        @insert_generic
        $scalar: ty
    ) => {
        $scalar
    };

    (
        @impl_trait
        impl< <$generic:tt> > $impl_trait:tt for $name:ty {
            $($body:tt)+
        }
    ) => {
        impl<$generic> $crate::$impl_trait<$generic> for $name
        where $generic: $crate::ScalarValue,
              for<'__b> &'__b $generic: $crate::ScalarRefValue<'__b>
        {
            $($body)+
        }
    };

    (
        @impl_trait
        impl<$scalar:ty> $impl_trait:tt for $name:ty {
            $($body:tt)+
        }
    ) => {
        impl $crate::$impl_trait<$scalar> for $name {
            $($body)+
        }
    };




    // Each of the @parse match arms accumulates data up to a call to @generate
    //
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
        graphql_scalar!(
            @impl_trait
            impl <$($scalar)+> GraphQLType for $name {
                type Context = ();
                type TypeInfo = ();

                fn name(_: &Self::TypeInfo) -> Option<&str> {
                    Some(graphql_scalar!(@as_expr $($outname)+))
                }

                fn meta<'r>(
                    info: &Self::TypeInfo,
                    registry: &mut $crate::Registry<'r, graphql_scalar!(@insert_generic $($scalar)+)>
                ) -> $crate::meta::MetaType<'r, graphql_scalar!(@insert_generic $($scalar)+)>
                where for<'__b> &'__b graphql_scalar!(@insert_generic $($scalar)+): $crate::ScalarRefValue<'__b>,
                    graphql_scalar!(@insert_generic $($scalar)+): 'r
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
                    _: Option<&[$crate::Selection<graphql_scalar!(@insert_generic $($scalar)+)>]>,
                    _: &$crate::Executor<graphql_scalar!(@insert_generic $($scalar)+), Self::Context>) -> $crate::Value<graphql_scalar!(@insert_generic $($scalar)+)> {
                    $resolve_body
                }
            });

        graphql_scalar!(
            @impl_trait
            impl<$($scalar)+> ToInputValue for $name {
                fn to_input_value(&$resolve_self_var) -> $crate::InputValue<graphql_scalar!(@insert_generic $($scalar)+)> {
                    let v = $resolve_body;
                    $crate::ToInputValue::to_input_value(&v)
                }
            }
        );

        graphql_scalar!(
            @impl_trait
            impl<$($scalar)+> FromInputValue for $name {
                fn from_input_value(
                    $from_input_value_arg: &$crate::InputValue<graphql_scalar!(@insert_generic $($scalar)+)>
                ) -> $from_input_value_result {
                    $from_input_value_body
                }
            }
        );

        graphql_scalar!(
            @impl_trait
            impl<$($scalar)+> ParseScalarValue for $name {
                fn from_str<$from_str_lt>($from_str_arg: $crate::parser::ScalarToken<$from_str_lt>) -> $from_str_result {
                    $from_str_body
                }
            }
        );

        impl $crate::FromInputValue for $name {
            fn from_input_value($fiv_arg: &$crate::InputValue) -> $fiv_result {
                $fiv_body
            }
        }
    };

    // No more items to parse
    (
        @parse,
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
        graphql_scalar!(
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
        @parse,
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
        @parse,
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
        @parse,
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
        @parse,
        meta = {$($meta:tt)*},
        $(resolve = {$($resolve_body:tt)+},)*
        $(from_input_value = {$($from_input_value_body:tt)+},)*
        $(from_str = {$($from_str_body:tt)+},)*
        rest = resolve(&$selfvar:ident) -> Value $body:block $($rest:tt)*
    ) => {
        graphql_scalar!(
            @parse,
            meta = {$($meta)*},
            resolve = {
                self_var = $selfvar,
                body = $body,
            },
            $(from_input_value = {$($from_input_value_body)+},)*
            $(from_str = {$($from_str_body)+},)*
            rest = $($rest)*
        );
    };

    // from_input_value(arg: &InputValue) -> ... { ... }
    (
        @parse,
        meta = { $($meta:tt)* },
        $(resolve = {$($resolve_body:tt)+})*,
        $(from_input_value = {$($from_input_value_body:tt)+},)*
        $(from_str = {$($from_str_body:tt)+},)*
        rest = from_input_value($arg:ident: &InputValue) -> $result:ty $body:block $($rest:tt)*
    ) => {
        graphql_scalar!(
            @parse,
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
        @parse,
        meta = { $($meta:tt)* },
        $(resolve = {$($resolve_body:tt)+},)*
        $(from_input_value = {$($from_input_value_body:tt)+},)*
        $(from_str = {$($from_str_body:tt)+},)*
        rest = from_str<$from_str_lt: tt>($value_arg:ident: ScalarToken<$ignored_lt2: tt>) -> $result:ty $body:block $($rest:tt)*
    ) => {
        graphql_scalar!(
            @parse,
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
        @parse,
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
        graphql_scalar!(
            @parse,
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

    // Entry point:
    // RustName as "GrahpQLName" where scalar
    ($name:ty as $outname:tt where Scalar = <$generic:tt> { $($items:tt)* }) => {
        graphql_scalar!(
            @parse,
            meta = {
                name = $name,
                outname = {$outname},
                scalar = {<$generic>},
            },
            rest = $($items)*
        );
    };

    ($name:ty as $outname:tt where Scalar = $scalar: ty { $($items:tt)* }) => {
        graphql_scalar!(
            @parse,
            meta = {
                name = $name,
                outname = {$outname},
                scalar = {$scalar},
            },
            rest = $($items)*
        );
    };
    // Entry point:
    // RustName as "GraphQLName" { ... }
    ( $name:ty as $outname:tt { $( $items:tt )* }) => {
        graphql_scalar!(
            @parse,
            meta = {
                name = $name,
                outname = {$outname},
                scalar = {<__S>},
            },
            rest = $($items)*
        );
    };


    ($name:ty where Scalar = <$generic:tt> { $($items:tt)* }) => {
        graphql_scalar!(
            @parse,
            meta = {
                name = $name,
                outname = {stringify!($name)},
                scalar = {<$generic>},
            },
            rest = $($items)*
        );
    };

    ($name:ty where Scalar = $scalar: ty { $($items:tt)* }) => {
        graphql_scalar!(
            @parse,
            meta = {
                name = $name,
                outname = {stringify!($name)},
                scalar = {$scalar},
            },
            rest = $($items)*
        )
    };
    // Entry point
    // RustName { ... }
    ( $name:ty { $( $items:tt )* }) => {
        graphql_scalar!(
            @parse,
            meta = {
                name = $name,
                outname = {stringify!($name)},
                scalar = {<__S>},
            },
            rest = $($items)*
        );
    };
}
