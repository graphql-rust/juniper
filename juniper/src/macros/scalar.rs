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
# use juniper::{Value, FieldResult};
struct UserID(String);

graphql_scalar!(UserID {
    description: "An opaque identifier, represented as a string"

    resolve(&self) -> Value {
        Value::string(&self.0)
    }

    from_input_value(v: &InputValue) -> Option<UserID> {
        v.as_string_value().map(|s| UserID(s.to_owned()))
    }
});

# fn main() { }
```

In addition to implementing `GraphQLType` for the type in question,
`FromInputValue` and `ToInputValue` is also implemented. This makes the type
usable as arguments and default values.

*/
#[macro_export]
macro_rules! graphql_scalar {
    ( @as_expr, $e:expr) => { $e };

    // Calls $val.$func($arg) if $arg is not None
    ( @maybe_apply, None, $func:ident, $val:expr ) => { $val };
    ( @maybe_apply, $arg:tt, $func:ident, $val:expr ) => { $val.$func($arg) };

    // Each of the @parse match arms accumulates data up to a call to @generate
    //
    // ( $name, $outname, $descr ): the name of the Rust type and the name of the
    // GraphQL scalar (as a string), and the description of the scalar (as a
    // string or none).
    //
    // ( $resolve_selfvar, $resolve_body ): the "self" argument and body for the
    // resolve() method on GraphQLType and the to_input_value() method on ToInputValue.
    //
    // ( $fiv_arg, $fiv_result, $fiv_body ): the method argument, result type,
    // and body for the from() method on FromInputValue.
    (
        @generate,
        ( $name:ty, $outname:expr, $descr:tt ),
        (
            ( $resolve_selfvar:ident, $resolve_body:block ),
            ( $fiv_arg:ident, $fiv_result:ty, $fiv_body:block )
        )
    ) => {
        impl $crate::GraphQLType for $name {
            type Context = ();
            type TypeInfo = ();

            fn name(_: &()) -> Option<&str> {
                Some(graphql_scalar!( @as_expr, $outname ))
            }

            fn meta<'r>(
                info: &(),
                registry: &mut $crate::Registry<'r>
            ) -> $crate::meta::MetaType<'r> {
                graphql_scalar!(
                    @maybe_apply, $descr, description,
                    registry.build_scalar_type::<Self>(info))
                    .into_meta()
            }

            fn resolve(
                &$resolve_selfvar,
                _: &(),
                _: Option<&[$crate::Selection]>,
                _: &$crate::Executor<Self::Context>) -> $crate::Value {
                $resolve_body
            }
        }

        impl $crate::ToInputValue for $name {
            fn to_input_value(&$resolve_selfvar) -> $crate::InputValue {
                $crate::ToInputValue::to_input_value(&$resolve_body)
            }
        }

        impl $crate::FromInputValue for $name {
            fn from_input_value($fiv_arg: &$crate::InputValue) -> $fiv_result {
                $fiv_body
            }
        }
    };

    // No more items to parse
    (
        @parse,
        $meta:tt,
        $acc:tt,
    ) => {
        graphql_scalar!( @generate, $meta, $acc );
    };

    // resolve(&self) -> Value { ... }
    (
        @parse,
        $meta:tt,
        ( $_ignored:tt, $fiv:tt ),
        resolve(&$selfvar:ident) -> Value $body:block $($rest:tt)*
    ) => {
        graphql_scalar!( @parse, $meta, ( ($selfvar, $body), $fiv ), $($rest)* );
    };

    // from_input_value(arg: &InputValue) -> ... { ... }
    (
        @parse,
        $meta:tt,
        ( $resolve:tt, $_ignored:tt ),
        from_input_value($arg:ident: &InputValue) -> $result:ty $body:block $($rest:tt)*
    ) => {
        graphql_scalar!( @parse, $meta, ( $resolve, ( $arg, $result, $body ) ), $($rest)* );
    };

    // description: <description>
    (
        @parse,
        ( $name:ty, $outname:expr, $_ignored:tt ),
        $acc:tt,
        description: $descr:tt $($rest:tt)*
    ) => {
        graphql_scalar!( @parse, ( $name, $outname, $descr ), $acc, $($rest)* );
    };

    // Entry point:
    // RustName as "GraphQLName" { ... }
    ( $name:ty as $outname:tt { $( $items:tt )* }) => {
        graphql_scalar!( @parse, ( $name, $outname, None ), ( None, None ), $($items)* );
    };

    // Entry point
    // RustName { ... }
    ( $name:ty { $( $items:tt )* }) => {
        graphql_scalar!( @parse, ( $name, stringify!($name), None ), ( None, None ), $($items)* );
    };
}
