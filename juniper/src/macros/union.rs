/**
Expose GraphQL unions

Like interfaces, mapping unions can be tricky in idiomatic Rust. Because of
their similarity, the helper macros are similar, too: you provide a set of
expressions that resolve the union into the actual concrete type.

## Syntax

See the documentation for [`graphql_object!`][1] on the general item and type
syntax. `graphql_union!` supports only `description` and `interface_resolvers`
items, no fields or interfaces can be declared.

See the documentation for [`graphql_interface!`][2] on the syntax for interface
resolvers.

[1]: macro.graphql_object!.html
[2]: macro.graphql_interface!.html
*/
#[macro_export(local_inner_macros)]
macro_rules! graphql_union {

    (
        @generate,
        meta = {
            lifetimes = [$($lifetimes:tt,)*],
            name = $name:ty,
            ctx = $ctx:ty,
            main_self = $main_self:ident,
            outname = {$($outname:tt)*},
            scalar = {$($scalar:tt)*},
            $(description = $desciption:tt,)*
            additional = {
                resolver = {
                    $(context = $resolver_ctx: ident,)*
                    items = [
                        $({
                            src = $resolver_src: ty,
                            resolver = $resolver_expr: expr,
                        },)*
                    ],
                 },
            },
        },
        items = [],
    ) => {
        __juniper_impl_trait!(
            impl<$($scalar)* $(, $lifetimes)* > GraphQLType for $name {
                type Context = $ctx;
                type TypeInfo = ();

                fn name(_ : &Self::TypeInfo) -> Option<&str> {
                    Some($($outname)*)
                }

                fn meta<'r>(
                    info: &Self::TypeInfo,
                    registry: &mut $crate::Registry<'r, __juniper_insert_generic!($($scalar)+)>
                ) -> $crate::meta::MetaType<'r, __juniper_insert_generic!($($scalar)+)>
                where for<'__b> &'__b __juniper_insert_generic!($($scalar)+): $crate::ScalarRefValue<'__b>,
                    __juniper_insert_generic!($($scalar)+): 'r
                {
                    let types = &[
                        $(
                          registry.get_type::<$resolver_src>(&()),
                        )*
                    ];
                    registry.build_union_type::<$name>(
                        info, types
                    )
                        $(.description($desciption))*
                        .into_meta()
                }

                #[allow(unused_variables)]
                fn concrete_type_name(&$main_self, context: &Self::Context, _info: &Self::TypeInfo) -> String {
                    $(let $resolver_ctx = &context;)*

                    $(
                        if ($resolver_expr as ::std::option::Option<$resolver_src>).is_some() {
                            return
                                <$resolver_src as $crate::GraphQLType<_>>::name(&()).unwrap().to_owned();
                        }
                    )*

                    __graphql__panic!("Concrete type not handled by instance resolvers on {}", $($outname)*);
                }

                fn resolve_into_type(
                    &$main_self,
                    _info: &Self::TypeInfo,
                    type_name: &str,
                    _: Option<&[$crate::Selection<__juniper_insert_generic!($($scalar)*)>]>,
                    executor: &$crate::Executor<Self::Context, __juniper_insert_generic!($($scalar)*)>,
                ) -> $crate::ExecutionResult<__juniper_insert_generic!($($scalar)*)> {
                    $(let $resolver_ctx = &executor.context();)*

                    $(
                        if type_name == (<$resolver_src as $crate::GraphQLType<_>>::name(&())).unwrap() {
                            return executor.resolve(&(), &$resolver_expr);
                        }
                    )*

                     __graphql__panic!("Concrete type not handled by instance resolvers on {}", $($outname)*);
                }
            }
        );
    };


    (
        @parse,
        meta = {$($meta:tt)*},
        rest = $($rest:tt)*
    ) => {
        __juniper_parse_field_list!(
            success_callback = graphql_union,
            additional_parser = {
                callback = __juniper_parse_instance_resolver,
                header = {},
            },
            meta = {$($meta)*},
            items = [],
            rest = $($rest)*
        );
    };
    (@$($stuff:tt)*) => {
        __graphql__compile_error!("Invalid syntax for `graphql_union!`");
    };

    ($($rest: tt)*) => {
        __juniper_parse_object_header!(
            callback = graphql_union,
            rest = $($rest)*
        );
    };
}
