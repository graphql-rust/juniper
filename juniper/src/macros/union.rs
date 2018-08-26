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
    ( @as_item, $i:item) => { $i };
    ( @as_expr, $e:expr) => { $e };
    ( @as_path, $p:path) => { $p };
    ( @as_type, $t:ty) => { $t };

    // description: <description>
    (
        @ gather_meta,
        ($reg:expr, $acc:expr, $descr:expr),
        description : $value:tt $( $rest:tt )*
    ) => {
        $descr = Some(graphql_interface!(@as_expr, $value));

        graphql_union!(@ gather_meta, ($reg, $acc, $descr), $( $rest )*)
    };

    // Gathering meta for instance resolvers
    // instance_resolvers: | <ctxtvar> | [...]
    (
        @ gather_meta,
        ($reg:expr, $acc:expr, $descr:expr),
        instance_resolvers: | $ctxtvar:pat
                            | { $( $srctype:ty => $resolver:expr ),* $(,)* } $( $rest:tt )*
    ) => {
        $acc = __graphql__vec![
            $(
                $reg.get_type::<$srctype>(&())
            ),*
        ];

        graphql_union!(@ gather_meta, ($reg, $acc, $descr), $( $rest )*)
    };

    // To generate the "concrete type name" resolver, syntax case:
    // instance_resolvers: | <ctxtvar> | [...]
    (
        @ concrete_type_name,
        ($outname:tt, $ctxtarg:ident, $ctxttype:ty),
        instance_resolvers: | $ctxtvar:pat
                            | { $( $srctype:ty => $resolver:expr ),* $(,)* } $( $rest:tt )*
    ) => {
        let $ctxtvar = &$ctxtarg;

        $(
            if let Some(_) = $resolver as Option<$srctype> {
                return (<$srctype as $crate::GraphQLType>::name(&())).unwrap().to_owned();
            }
        )*

            __graphql__panic!("Concrete type not handled by instance resolvers on {}", $outname);
    };

    // To generate the "resolve into type" resolver, syntax case:
    // instance_resolvers: | <ctxtvar> | [...]
    (
        @ resolve_into_type,
        ($outname:tt, $typenamearg:ident, $execarg:ident, $ctxttype:ty),
        instance_resolvers: | $ctxtvar:pat
                            | { $( $srctype:ty => $resolver:expr ),* $(,)* } $( $rest:tt )*
    ) => {
        let $ctxtvar = &$execarg.context();

        $(
            if $typenamearg == (<$srctype as $crate::GraphQLType>::name(&())).unwrap().to_owned() {
                return $execarg.resolve(&(), &$resolver);
            }
        )*

           __graphql__panic!("Concrete type not handled by instance resolvers on {}", $outname);
    };

    // eat commas
    ( @ $mfn:ident, $args:tt, , $($rest:tt)* ) => {
        graphql_union!(@ $mfn, $args, $($rest)*);
    };

    // eat one tt
    ( @ $mfn:ident, $args:tt, $item:tt $($rest:tt)* ) => {
        graphql_union!(@ $mfn, $args, $($rest)*);
    };

    // end case
    ( @ $mfn:ident, $args:tt, ) => {};

    (
        ( $($lifetime:tt),* ) $name:ty : $ctxt:ty as $outname:tt | &$mainself:ident | {
            $( $items:tt )*
        }
    ) => {
        graphql_union!(@as_item, impl<$($lifetime)*> $crate::GraphQLType for $name {
            type Context = $ctxt;
            type TypeInfo = ();

            fn name(_: &()) -> Option<&str> {
                Some($outname)
            }

            #[allow(unused_assignments)]
            #[allow(unused_mut)]
            fn meta<'r>(_: &(), registry: &mut $crate::Registry<'r>) -> $crate::meta::MetaType<'r> {
                let mut types;
                let mut description = None;
                graphql_union!(@ gather_meta, (registry, types, description), $($items)*);
                let mut mt = registry.build_union_type::<$name>(&(), &types);

                if let Some(description) = description {
                    mt = mt.description(description);
                }

                mt.into_meta()
            }

            fn concrete_type_name(&$mainself, context: &Self::Context, _: &()) -> String {
                graphql_union!(
                    @ concrete_type_name,
                    ($outname, context, $ctxt),
                    $($items)*);
            }

            fn resolve_into_type(
                &$mainself,
                _: &(),
                type_name: &str,
                _: Option<&[$crate::Selection]>,
                executor: &$crate::Executor<Self::Context>,
            )
                -> $crate::ExecutionResult
            {
                graphql_union!(
                    @ resolve_into_type,
                    ($outname, type_name, executor, $ctxt),
                    $($items)*);
            }
        });
    };

    (
        <$($lifetime:tt),*> $name:ty : $ctxt:ty as $outname:tt | &$mainself:ident | {
            $( $items:tt )*
        }
    ) => {
        graphql_union!(
            ($($lifetime),*) $name : $ctxt as $outname | &$mainself | { $( $items )* });
    };

    (
        $name:ty : $ctxt:ty as $outname:tt | &$mainself:ident | {
            $( $items:tt )*
        }
    ) => {
        graphql_union!(() $name : $ctxt as $outname | &$mainself | { $( $items )* });
    };

    (
        $name:ty : $ctxt:ty | &$mainself:ident | {
            $( $items:tt )*
        }
    ) => {
        graphql_union!(() $name : $ctxt as (__graphql__stringify!($name)) | &$mainself | { $( $items )* });
    };
}
