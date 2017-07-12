/**
Expose simple enums

GraphQL enums are similar to enums classes C++ - more like grouped constants
with type safety than what Rust enums offer. This macro can be used to export
non-data carrying Rust enums to GraphQL:

```rust
# #[macro_use] extern crate juniper;
enum Color {
    Red,
    Orange,
    Green,
    Blue,
    Black,
}

graphql_enum!(Color {
    Color::Red => "RED" as "The color red",
    Color::Orange => "ORANGE",
    Color::Green => "GREEN",
    Color::Blue => "BLUE",
    Color::Black => "BLACK" deprecated "Superseded by ORANGE",
});

# fn main() { }
```

The macro expands to a `match` statement which will result in a compilation
error if not all enum variants are covered. It also creates an implementation
for `FromInputValue` and `ToInputValue`, making it usable in arguments and
default values.

If you want to expose the enum under a different name than the Rust type,
you can write `graphql_enum!(Color as "MyColor" { ...`.

*/
#[macro_export]
macro_rules! graphql_enum {
    ( @as_expr, $e:expr) => { $e };
    ( @as_pattern, $p:pat) => { $p };
    ( @as_path, $p:path) => { $p };

    // Calls $val.$func($arg) if $arg is not None
    ( @maybe_apply, None, $func:ident, $val:expr ) => { $val };
    ( @maybe_apply, $arg:tt, $func:ident, $val:expr ) => { $val.$func($arg) };

    // Each of the @parse match arms accumulates data up to a call to @generate.
    //
    // ( $name, $outname, $descr ): the name of the Rust enum, the name of the
    // GraphQL enum (as a string), and the description of the enum (as a string or None)
    //
    // [ ( $eval, $ename, $edescr, $edepr ) , ] the value of the Rust enum,
    // the value of the GraphQL enum (as a string), the description of the enum
    // value (as a string or None), and the deprecation reason of the enum value
    // (as a string or None).
    (
        @generate,
        ( $name:path, $outname:tt, $descr:tt ),
        [ $( ( $eval:tt, $ename:tt, $edescr:tt, $edepr:tt ) , )* ]
    ) => {
        impl $crate::GraphQLType for $name {
            type Context = ();
            type TypeInfo = ();

            fn name(_: &()) -> Option<&str> {
                Some(graphql_enum!(@as_expr, $outname))
            }

            fn meta<'r>(info: &(), registry: &mut $crate::Registry<'r>) -> $crate::meta::MetaType<'r> {
                graphql_enum!(
                    @maybe_apply, $descr, description,
                    registry.build_enum_type::<$name>(info, &[
                        $(
                            graphql_enum!(
                                @maybe_apply,
                                $edepr, deprecated,
                                graphql_enum!(
                                    @maybe_apply,
                                    $edescr, description,
                                    $crate::meta::EnumValue::new(graphql_enum!(@as_expr, $ename))))
                        ),*
                    ]))
                    .into_meta()
            }

            fn resolve(&self, _: &(), _: Option<&[$crate::Selection]>, _: &$crate::Executor<Self::Context>) -> $crate::Value {
                match *self {
                    $(
                        graphql_enum!(@as_pattern, $eval) =>
                            $crate::Value::string(graphql_enum!(@as_expr, $ename)) ),*
                }
            }
        }

        impl $crate::FromInputValue for $name {
            fn from(v: &$crate::InputValue) -> Option<$name> {
                match v.as_enum_value().or_else(|| v.as_string_value()) {
                    $(
                        Some(graphql_enum!(@as_pattern, $ename))
                            => Some(graphql_enum!(@as_expr, $eval)), )*
                    _ => None,
                }
            }
        }

        impl $crate::ToInputValue for $name {
            fn to(&self) -> $crate::InputValue {
                match *self {
                    $(
                        graphql_enum!(@as_pattern, $eval) =>
                            $crate::InputValue::string(graphql_enum!(@as_expr, $ename)) ),*
                }
            }
        }
    };

    // No more items to parse
    ( @parse, $meta:tt, $acc:tt, ) => {
        graphql_enum!( @generate, $meta, $acc );
    };

    // Remove extraneous commas
    ( @parse, $meta:tt, $acc:tt, , $($rest:tt)* ) => {
        graphql_enum!( @parse, $meta, $acc, $($rest)* );
    };

    // description: <description>
    (
        @parse,
        ( $name:tt, $outname:tt, $_ignore:tt ),
        $acc:tt,
        description: $descr:tt $($items:tt)*
    ) => {
        graphql_enum!( @parse, ( $name, $outname, $descr ), $acc, $($items)* );
    };

    // RustEnumValue => "GraphQL enum value" deprecated <reason>
    (
        @parse,
        $meta:tt,
        [ $($acc:tt ,)* ],
        $eval:path => $ename:tt deprecated $depr:tt $($rest:tt)*
    ) => {
        graphql_enum!( @parse, $meta, [ $($acc ,)* ( $eval, $ename, None, $depr ), ], $($rest)* );
    };

    // RustEnumValue => "GraphQL enum value" as <description> deprecated <reason>
    (
        @parse,
        $meta:tt,
        [ $($acc:tt ,)* ],
        $eval:path => $ename:tt as $descr:tt deprecated $depr:tt $($rest:tt)*
    ) => {
        graphql_enum!( @parse, $meta, [ $($acc ,)* ( $eval, $ename, $descr, $depr ), ], $($rest)* );
    };

    // RustEnumValue => "GraphQL enum value" as <description>
    (
        @parse,
        $meta:tt,
        [ $($acc:tt ,)* ],
        $eval:path => $ename:tt as $descr:tt $($rest:tt)*
    ) => {
        graphql_enum!( @parse, $meta, [ $($acc ,)* ( $eval, $ename, $descr, None ), ], $($rest)* );
    };

    // RustEnumValue => "GraphQL enum value"
    (
        @parse,
        $meta:tt,
        [ $($acc:tt ,)* ],
        $eval:path => $ename:tt $($rest:tt)*
    ) => {
        graphql_enum!( @parse, $meta, [ $($acc ,)* ( $eval , $ename , None , None ), ], $($rest)* );
    };

    // Entry point:
    // RustEnumName as "GraphQLEnumName" { ... }
    (
        $name:path as $outname:tt { $($items:tt)* }
    ) => {
        graphql_enum!( @parse, ( $name, $outname, None ), [ ], $($items)* );
    };

    // Entry point
    // RustEnumName { ... }
    (
        $name:path { $($items:tt)* }
    ) => {
        graphql_enum!( @parse, ( $name, (stringify!($name)), None ), [ ], $($items)* );
    };
}
