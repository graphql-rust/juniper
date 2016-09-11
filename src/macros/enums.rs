/**
Expose simple enums

GraphQL enums are similar to enums classes C++ - more like grouped constants
with type safety than what Rust enums offer. This macro can be used to export
non-data carrying Rust enums to GraphQL:

```rust
# #[macro_use] extern crate juniper;
enum Color {
    Red,
    Green,
    Blue
}

graphql_enum!(Color {
    Color::Red => "RED",
    Color::Green => "GREEN",
    Color::Blue => "BLUE",
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

    // EnumName as "__ExportedNmae" { Enum::Value => "STRING_VALUE", }
    // with no trailing comma
    ( $name:path as $outname:tt { $($eval:path => $ename:tt),* }) => {
        impl<CtxT> $crate::GraphQLType<CtxT> for $name {
            fn name() -> Option<&'static str> {
                Some(graphql_enum!(@as_expr, $outname))
            }

            fn meta(registry: &mut $crate::Registry<CtxT>) -> $crate::meta::MetaType {
                registry.build_enum_type::<$name>()(&[
                        $( $crate::meta::EnumValue::new(graphql_enum!(@as_expr, $ename)) ),*
                    ])
                    .into_meta()
            }

            fn resolve(&self, _: Option<Vec<$crate::Selection>>, _: &mut $crate::Executor<CtxT>) -> $crate::Value {
                match self {
                    $(
                        &graphql_enum!(@as_pattern, $eval) =>
                            $crate::Value::string(graphql_enum!(@as_expr, $ename)) ),*
                }
            }
        }

        impl $crate::FromInputValue for $name {
            fn from(v: &$crate::InputValue) -> Option<$name> {
                match v.as_enum_value() {
                    $(
                        Some(graphql_enum!(@as_pattern, $ename))
                            => Some(graphql_enum!(@as_expr, $eval)), )*
                    _ => None,
                }
            }
        }

        impl $crate::ToInputValue for $name {
            fn to(&self) -> $crate::InputValue {
                match self {
                    $(
                        &graphql_enum!(@as_pattern, $eval) =>
                            $crate::InputValue::string(graphql_enum!(@as_expr, $ename)) ),*
                }
            }
        }
    };

    // Same as above, *with* trailing comma
    ( $name:path as $outname:tt { $($eval:path => $ename:tt, )* }) => {
        graphql_enum!($name as $outname { $( $eval => $ename ),* });
    };

    // Default named enum, without trailing comma
    ( $name:path { $($eval:path => $ename:tt),* }) => {
        graphql_enum!($name as (stringify!($name)) { $( $eval => $ename ),* });
    };

    // Default named enum, with trailing comma
    ( $name:path { $($eval:path => $ename:tt, )* }) => {
        graphql_enum!($name as (stringify!($name)) { $( $eval => $ename ),* });
    };
}
