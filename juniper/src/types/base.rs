use crate::{
    ast::{Directive, FromInputValue, InputValue, Selection},
    executor::{ExecutionResult, Executor, Registry, Variables},
    parser::Spanning,
    schema::meta::{Argument, MetaType},
    value::{DefaultScalarValue, Object, ScalarValue, Value},
    BoxFuture,
};
use indexmap::IndexMap;
use juniper_codegen::GraphQLEnumInternal as GraphQLEnum;

/// GraphQL type kind
///
/// The GraphQL specification defines a number of type kinds - the meta type\
/// of a type.
#[derive(Clone, Eq, PartialEq, Debug, GraphQLEnum)]
#[graphql(name = "__TypeKind")]
pub enum TypeKind {
    /// ## Scalar types
    ///
    /// Scalar types appear as the leaf nodes of GraphQL queries. Strings,\
    /// numbers, and booleans are the built in types, and while it's possible\
    /// to define your own, it's relatively uncommon.
    Scalar,

    /// ## Object types
    ///
    /// The most common type to be implemented by users. Objects have fields\
    /// and can implement interfaces.
    Object,

    /// ## Interface types
    ///
    /// Interface types are used to represent overlapping fields between\
    /// multiple types, and can be queried for their concrete type.
    Interface,

    /// ## Union types
    ///
    /// Unions are similar to interfaces but can not contain any fields on\
    /// their own.
    Union,

    /// ## Enum types
    ///
    /// Like scalars, enum types appear as the leaf nodes of GraphQL queries.
    Enum,

    /// ## Input objects
    ///
    /// Represents complex values provided in queries _into_ the system.
    #[graphql(name = "INPUT_OBJECT")]
    InputObject,

    /// ## List types
    ///
    /// Represent lists of other types. This library provides implementations\
    /// for vectors and slices, but other Rust types can be extended to serve\
    /// as GraphQL lists.
    List,

    /// ## Non-null types
    ///
    /// In GraphQL, nullable types are the default. By putting a `!` after a\
    /// type, it becomes non-nullable.
    #[graphql(name = "NON_NULL")]
    NonNull,
}

/// Field argument container
#[derive(Debug)]
pub struct Arguments<'a, S = DefaultScalarValue> {
    args: Option<IndexMap<&'a str, InputValue<S>>>,
}

impl<'a, S> Arguments<'a, S>
where
    S: ScalarValue,
{
    #[doc(hidden)]
    pub fn new(
        mut args: Option<IndexMap<&'a str, InputValue<S>>>,
        meta_args: &'a Option<Vec<Argument<S>>>,
    ) -> Self {
        if meta_args.is_some() && args.is_none() {
            args = Some(IndexMap::new());
        }

        if let (&mut Some(ref mut args), &Some(ref meta_args)) = (&mut args, meta_args) {
            for arg in meta_args {
                if !args.contains_key(arg.name.as_str()) || args[arg.name.as_str()].is_null() {
                    if let Some(ref default_value) = arg.default_value {
                        args.insert(arg.name.as_str(), default_value.clone());
                    } else {
                        args.insert(arg.name.as_str(), InputValue::null());
                    }
                }
            }
        }

        Arguments { args }
    }

    /// Get and convert an argument into the desired type.
    ///
    /// If the argument is found, or a default argument has been provided,
    /// the `InputValue` will be converted into the type `T`.
    ///
    /// Returns `Some` if the argument is present _and_ type conversion
    /// succeeeds.
    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: FromInputValue<S>,
    {
        match self.args {
            Some(ref args) => match args.get(key) {
                Some(v) => v.convert(),
                None => None,
            },
            None => None,
        }
    }
}

/**
Primary trait used to expose Rust types in a GraphQL schema

All of the convenience macros ultimately expand into an implementation of
this trait for the given type. The macros remove duplicated definitions of
fields and arguments, and add type checks on all resolve functions
automatically. This can all be done manually.

`GraphQLType` provides _some_ convenience methods for you, in the form of
optional trait methods. The `name` and `meta` methods are mandatory, but
other than that, it depends on what type you're exposing:

* Scalars, enums, lists and non null wrappers only require `resolve`,
* Interfaces and objects require `resolve_field` _or_ `resolve` if you want
  to implement custom resolution logic (probably not),
* Interfaces and unions require `resolve_into_type` and `concrete_type_name`.
* Input objects do not require anything

## Example

Manually deriving an object is straightforward but tedious. This is the
equivalent of the `User` object as shown in the example in the documentation
root:

```rust
use juniper::{GraphQLType, Registry, FieldResult, Context,
              Arguments, Executor, ExecutionResult,
              DefaultScalarValue, BoxFuture};
use juniper::meta::MetaType;
# use std::collections::HashMap;

#[derive(Debug)]
struct User { id: String, name: String, friend_ids: Vec<String>  }
#[derive(Debug)]
struct Database { users: HashMap<String, User> }

impl Context for Database {}

impl GraphQLType<DefaultScalarValue> for User
{
    type Context = Database;
    type TypeInfo = ();

    fn name(_: &()) -> Option<&'static str> {
        Some("User")
    }

    fn meta<'r>(_: &(), registry: &mut Registry<'r>) -> MetaType<'r>
    where DefaultScalarValue: 'r,
    {
        // First, we need to define all fields and their types on this type.
        //
        // If we need arguments, want to implement interfaces, or want to add
        // documentation strings, we can do it here.
        let fields = &[
            registry.field::<&String>("id", &()),
            registry.field::<&String>("name", &()),
            registry.field::<Vec<&User>>("friends", &()),
        ];

        registry.build_object_type::<User>(&(), fields).into_meta()
    }

    fn resolve_field<'me, 'ty, 'field, 'args, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        field_name: &'field str,
        arguments: &'args Arguments<'args, DefaultScalarValue>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, DefaultScalarValue>,
    ) -> BoxFuture<'fut, ExecutionResult<DefaultScalarValue>>
    where
        'me: 'fut,
        'ty: 'fut,
        'args: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
        'field: 'fut,
        DefaultScalarValue: 'fut,
    {
        let f = async move {
            // Next, we need to match the queried field name. All arms of this
            // match statement return `ExecutionResult`, which makes it hard to
            // statically verify that the type you pass on to `executor.resolve*`
            // actually matches the one that you defined in `meta()` above.
            let database = executor.context();
            match field_name {
                // Because scalars are defined with another `Context` associated
                // type, you must use resolve_with_ctx here to make the executor
                // perform automatic type conversion of its argument.
                "id" => executor.resolve_with_ctx(info, &self.id).await,
                "name" => executor.resolve_with_ctx(info, &self.name).await,

                // You pass a vector of User objects to `executor.resolve`, and it
                // will determine which fields of the sub-objects to actually
                // resolve based on the query. The executor instance keeps track
                // of its current position in the query.
                "friends" => executor.resolve(info,
                    &self.friend_ids.iter()
                        .filter_map(|id| database.users.get(id))
                        .collect::<Vec<_>>()
                ).await,

                // We can only reach this panic in two cases; either a mismatch
                // between the defined schema in `meta()` above, or a validation
                // in this library failed because of a bug.
                //
                // In either of those two cases, the only reasonable way out is
                // to panic the thread.
                _ => panic!("Field {} not found on type User", field_name),
            }
        };
        futures::future::FutureExt::boxed(f)
    }
}
```

 */

pub trait GraphQLType<S>: Sized + Send + Sync
where
    S: ScalarValue,
{
    /// The expected context type for this GraphQL type
    ///
    /// The context is threaded through query execution to all affected nodes,
    /// and can be used to hold common data, e.g. database connections or
    /// request session information.
    type Context: Send + Sync;

    /// Type that may carry additional schema information
    ///
    /// This can be used to implement a schema that is partly dynamic,
    /// meaning that it can use information that is not known at compile time,
    /// for instance by reading it from a configuration file at start-up.
    type TypeInfo: Send + Sync;

    /// The name of the GraphQL type to expose.
    ///
    /// This function will be called multiple times during schema construction.
    /// It must _not_ perform any calculation and _always_ return the same
    /// value.
    fn name(info: &Self::TypeInfo) -> Option<&str>;

    /// The meta type representing this GraphQL type.
    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r;

    /// Return the concrete type name for this instance/union.
    ///
    /// The default implementation panics.
    #[allow(unused_variables)]
    fn concrete_type_name(&self, context: &Self::Context, info: &Self::TypeInfo) -> String {
        panic!("concrete_type_name must be implemented by unions and interfaces");
    }

    /// Resolve the value of a single field on this type.
    ///
    /// The arguments object contain all specified arguments, with default
    /// values substituted for the ones not provided by the query.
    ///
    /// The executor can be used to drive selections into sub-objects.
    ///
    /// The default implementation panics.
    #[allow(unused_variables)]
    fn resolve_field<'me, 'ty, 'field, 'args, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        field_name: &'field str,
        arguments: &'args Arguments<'args, S>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'args: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
        'field: 'fut,
        S: 'fut,
    {
        panic!("resolve_field must be implemented by object types");
    }

    /// Resolve this interface or union into a concrete type
    ///
    /// Try to resolve the current type into the type name provided. If the
    /// type matches, pass the instance along to `executor.resolve`.
    ///
    /// The default implementation panics.
    #[allow(unused_variables)]
    fn resolve_into_type<'me, 'ty, 'name, 'set, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        type_name: &'name str,
        selection_set: Option<&'set [Selection<'set, S>]>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'name: 'fut,
        'set: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
    {
        let f = async move {
            if Self::name(info).unwrap() == type_name {
                self.resolve(info, selection_set, executor).await
            } else {
                panic!("resolve_into_type must be implemented by unions and interfaces");
            }
        };

        futures::future::FutureExt::boxed(f)
    }

    /// Resolve the provided selection set against the current object.
    ///
    /// For non-object types, the selection set will be `None` and the value
    /// of the object should simply be returned.
    ///
    /// For objects, all fields in the selection set should be resolved.
    /// The default implementation uses `resolve_field` to resolve all fields,
    /// including those through fragment expansion.
    ///
    /// Since the GraphQL spec specificies that errors during field processing
    /// should result in a null-value, this might return Ok(Null) in case of
    /// failure. Errors are recorded internally.
    fn resolve<'me, 'ty, 'name, 'set, 'ref_err, 'err, 'fut>(
        &'me self,
        info: &'ty Self::TypeInfo,
        selection_set: Option<&'set [Selection<'set, S>]>,
        executor: &'ref_err Executor<'ref_err, 'err, Self::Context, S>,
    ) -> BoxFuture<'fut, ExecutionResult<S>>
    where
        'me: 'fut,
        'ty: 'fut,
        'name: 'fut,
        'set: 'fut,
        'ref_err: 'fut,
        'err: 'fut,
        S: 'fut,
    {
        let f = async move {
            if let Some(selection_set) = selection_set {
                let value = resolve_selection_set_into(self, info, selection_set, executor).await;
                Ok(value)
            } else {
                panic!("resolve() must be implemented by non-object output types");
            }
        };

        futures::future::FutureExt::boxed(f)
    }
}

struct AsyncField<S> {
    name: String,
    value: Option<Value<S>>,
}

enum AsyncValue<S> {
    Field(AsyncField<S>),
    Nested(Value<S>),
}

// Wrapper function around resolve_selection_set_into_async_recursive.
// This wrapper is necessary because async fns can not be recursive.
pub(crate) fn resolve_selection_set_into<'a, 'e, T, CtxT, S>(
    instance: &'a T,
    info: &'a T::TypeInfo,
    selection_set: &'e [Selection<'e, S>],
    executor: &'e Executor<'e, 'e, CtxT, S>,
) -> crate::BoxFuture<'a, Value<S>>
where
    T: GraphQLType<S, Context = CtxT>,
    T::TypeInfo: Send + Sync,
    S: ScalarValue,
    CtxT: Send + Sync,
    'e: 'a,
{
    Box::pin(resolve_selection_set_into_recursive(
        instance,
        info,
        selection_set,
        executor,
    ))
}

/// Resolver logic for queries'/mutations' selection set.
/// Calls appropriate resolver method for each field or fragment found
/// and then merges returned values into `result` or pushes errors to
/// field's/fragment's sub executor.
async fn resolve_selection_set_into_recursive<'a, T, CtxT, S>(
    instance: &'a T,
    info: &'a T::TypeInfo,
    selection_set: &'a [Selection<'a, S>],
    executor: &'a Executor<'a, 'a, CtxT, S>,
) -> Value<S>
where
    T: GraphQLType<S, Context = CtxT> + Send + Sync,
    T::TypeInfo: Send + Sync,
    S: ScalarValue,
    CtxT: Send + Sync,
{
    use futures::stream::{FuturesOrdered, StreamExt as _};

    let mut object = Object::with_capacity(selection_set.len());

    let mut async_values = FuturesOrdered::<crate::BoxFuture<'a, AsyncValue<S>>>::new();

    let meta_type = executor
        .schema()
        .concrete_type_by_name(
            T::name(info)
                .expect("Resolving named type's selection set")
                .as_ref(),
        )
        .expect("Type not found in schema");

    for selection in selection_set {
        match *selection {
            Selection::Field(Spanning {
                item: ref f,
                start: ref start_pos,
                ..
            }) => {
                if is_excluded(&f.directives, executor.variables()) {
                    continue;
                }

                let response_name = f.alias.as_ref().unwrap_or(&f.name).item;

                if f.name.item == "__typename" {
                    object.add_field(
                        response_name,
                        Value::scalar(instance.concrete_type_name(executor.context(), info)),
                    );
                    continue;
                }

                let meta_field = meta_type.field_by_name(f.name.item).unwrap_or_else(|| {
                    panic!(format!(
                        "Field {} not found on type {:?}",
                        f.name.item,
                        meta_type.name()
                    ))
                });

                let exec_vars = executor.variables();

                let sub_exec = executor.field_sub_executor(
                    &response_name,
                    f.name.item,
                    start_pos.clone(),
                    f.selection_set.as_ref().map(|v| &v[..]),
                );
                let args = Arguments::new(
                    f.arguments.as_ref().map(|m| {
                        m.item
                            .iter()
                            .map(|&(ref k, ref v)| (k.item, v.item.clone().into_const(exec_vars)))
                            .collect()
                    }),
                    &meta_field.arguments,
                );

                let pos = *start_pos;
                let is_non_null = meta_field.field_type.is_non_null();

                let response_name = response_name.to_string();
                let field_future = async move {
                    // TODO: implement custom future type instead of
                    //       two-level boxing.
                    let res = instance
                        .resolve_field(info, f.name.item, &args, &sub_exec)
                        .await;

                    let value = match res {
                        Ok(Value::Null) if is_non_null => None,
                        Ok(v) => Some(v),
                        Err(e) => {
                            sub_exec.push_error_at(e, pos);

                            if is_non_null {
                                None
                            } else {
                                Some(Value::null())
                            }
                        }
                    };
                    AsyncValue::Field(AsyncField {
                        name: response_name,
                        value,
                    })
                };
                async_values.push(Box::pin(field_future));
            }
            Selection::FragmentSpread(Spanning {
                item: ref spread, ..
            }) => {
                if is_excluded(&spread.directives, executor.variables()) {
                    continue;
                }

                // TODO: prevent duplicate boxing.
                let f = async move {
                    let fragment = &executor
                        .fragment_by_name(spread.name.item)
                        .expect("Fragment could not be found");
                    let value = resolve_selection_set_into(
                        instance,
                        info,
                        &fragment.selection_set[..],
                        executor,
                    )
                    .await;
                    AsyncValue::Nested(value)
                };
                async_values.push(Box::pin(f));
            }
            Selection::InlineFragment(Spanning {
                item: ref fragment,
                start: ref start_pos,
                ..
            }) => {
                if is_excluded(&fragment.directives, executor.variables()) {
                    continue;
                }

                let sub_exec = executor.type_sub_executor(
                    fragment.type_condition.as_ref().map(|c| c.item),
                    Some(&fragment.selection_set[..]),
                );

                if let Some(ref type_condition) = fragment.type_condition {
                    let sub_result = instance
                        .resolve_into_type(
                            info,
                            type_condition.item,
                            Some(&fragment.selection_set[..]),
                            &sub_exec,
                        )
                        .await;

                    if let Ok(Value::Object(obj)) = sub_result {
                        for (k, v) in obj {
                            // TODO: prevent duplicate boxing.
                            let f = async move {
                                AsyncValue::Field(AsyncField {
                                    name: k,
                                    value: Some(v),
                                })
                            };
                            async_values.push(Box::pin(f));
                        }
                    } else if let Err(e) = sub_result {
                        sub_exec.push_error_at(e, start_pos.clone());
                    }
                } else {
                    let f = async move {
                        let value = resolve_selection_set_into(
                            instance,
                            info,
                            &fragment.selection_set[..],
                            &sub_exec,
                        )
                        .await;
                        AsyncValue::Nested(value)
                    };
                    async_values.push(Box::pin(f));
                }
            }
        }
    }

    while let Some(item) = async_values.next().await {
        match item {
            AsyncValue::Field(AsyncField { name, value }) => {
                if let Some(value) = value {
                    merge_key_into(&mut object, &name, value);
                } else {
                    return Value::null();
                }
            }
            AsyncValue::Nested(obj) => match obj {
                v @ Value::Null => {
                    return v;
                }
                Value::Object(obj) => {
                    for (k, v) in obj {
                        merge_key_into(&mut object, &k, v);
                    }
                }
                _ => unreachable!(),
            },
        }
    }

    Value::Object(object)
}

pub(super) fn is_excluded<S>(
    directives: &Option<Vec<Spanning<Directive<S>>>>,
    vars: &Variables<S>,
) -> bool
where
    S: ScalarValue,
{
    if let Some(ref directives) = *directives {
        for &Spanning {
            item: ref directive,
            ..
        } in directives
        {
            let condition: bool = directive
                .arguments
                .iter()
                .flat_map(|m| m.item.get("if"))
                .flat_map(|v| v.item.clone().into_const(vars).convert())
                .next()
                .unwrap();

            if (directive.name.item == "skip" && condition)
                || (directive.name.item == "include" && !condition)
            {
                return true;
            }
        }
    }
    false
}

/// Merges `response_name`/`value` pair into `result`
pub(crate) fn merge_key_into<S>(result: &mut Object<S>, response_name: &str, value: Value<S>) {
    if let Some(&mut (_, ref mut e)) = result
        .iter_mut()
        .find(|&&mut (ref key, _)| key == response_name)
    {
        match *e {
            Value::Object(ref mut dest_obj) => {
                if let Value::Object(src_obj) = value {
                    merge_maps(dest_obj, src_obj);
                }
            }
            Value::List(ref mut dest_list) => {
                if let Value::List(src_list) = value {
                    dest_list
                        .iter_mut()
                        .zip(src_list.into_iter())
                        .for_each(|(d, s)| {
                            if let Value::Object(ref mut d_obj) = *d {
                                if let Value::Object(s_obj) = s {
                                    merge_maps(d_obj, s_obj);
                                }
                            }
                        });
                }
            }
            _ => {}
        }
        return;
    }
    result.add_field(response_name, value);
}

/// Merges `src` object's fields into `dest`
fn merge_maps<S>(dest: &mut Object<S>, src: Object<S>) {
    for (key, value) in src {
        if dest.contains_field(&key) {
            merge_key_into(dest, &key, value);
        } else {
            dest.add_field(key, value);
        }
    }
}
