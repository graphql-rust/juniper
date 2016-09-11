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

graphql_scalar!(UserID as "UserID" {
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

`graphql_scalar!` supports generic and lifetime parameters similar to
`graphql_object!`.

*/
#[macro_export]
macro_rules! graphql_scalar {
    (
        @build_scalar_resolver,
        resolve(&$selfvar:ident) -> Value $body:block $($rest:tt)*
    ) => {
        fn resolve(&$selfvar, _: Option<Vec<$crate::Selection>>, _: &mut $crate::Executor<CtxT>) -> $crate::Value {
            $body
        }
    };

    (
        @build_scalar_conv_impl,
        $name:ty; [$($lifetime:tt),*];
        resolve(&$selfvar:ident) -> Value $body:block $($rest:tt)*
    ) => {
        impl<$($lifetime),*> $crate::ToInputValue for $name {
            fn to(&$selfvar) -> $crate::InputValue {
                $crate::ToInputValue::to(&$body)
            }
        }

        graphql_scalar!(@build_scalar_conv_impl, $name; [$($lifetime),*]; $($rest)*);
    };

    (
        @build_scalar_conv_impl,
        $name:ty; [$($lifetime:tt),*];
        from_input_value($arg:ident: &InputValue) -> $result:ty $body:block
        $($rest:tt)*
    ) => {
        impl<$($lifetime),*> $crate::FromInputValue for $name {
            fn from($arg: &$crate::InputValue) -> $result {
                $body
            }
        }

        graphql_scalar!(@build_scalar_conv_impl, $name; [$($lifetime),*]; $($rest)*);
    };

    (
        @build_scalar_conv_impl,
        $name:ty; $($lifetime:tt),*;
    ) => {
    };

    (($($lifetime:tt),*) $name:ty as $outname:expr => { $( $items:tt )* }) => {
        impl<$($lifetime,)* CtxT> $crate::GraphQLType<CtxT> for $name {
            fn name() -> Option<&'static str> {
                Some($outname)
            }

            fn meta(registry: &mut $crate::Registry<CtxT>) -> $crate::meta::MetaType {
                registry.build_scalar_type::<Self>().into_meta()
            }

            graphql_scalar!(@build_scalar_resolver, $($items)*);
        }

        graphql_scalar!(@build_scalar_conv_impl, $name; [$($lifetime),*]; $($items)*);
    };

    (<$($lifetime:tt),*> $name:ty as $outname:tt { $( $items:tt )* }) => {
        graphql_scalar!(($($lifetime),*) $name as $outname => { $( $items )* });
    };

    ( $name:ty as $outname:tt { $( $items:tt )* }) => {
        graphql_scalar!(() $name as $outname => { $( $items )* });
    }
}
