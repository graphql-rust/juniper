use std::rc::Rc;

use indexmap::IndexMap;

use juniper_codegen::GraphQLEnumInternal as GraphQLEnum;

use crate::{
    ast::{Directive, FromInputValue, InputValue, Selection},
    executor::{ExecutionResult, Executor, Registry, ValuesIterator, Variables},
    parser::Spanning,
    schema::meta::{Argument, MetaType},
    value::{DefaultScalarValue, Object, ScalarRefValue, ScalarValue, Value},
    FieldError,
};

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
        for<'b> &'b S: ScalarRefValue<'b>,
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

Synchronous query/mutation related convenience macros ultimately expand into an implementation of
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
              DefaultScalarValue};
use juniper::meta::MetaType;
# use std::collections::HashMap;

#[derive(Debug)]
struct User { id: String, name: String, friend_ids: Vec<String>  }
#[derive(Debug)]
struct Database { users: HashMap<String, User> }

impl Context for Database {}

impl GraphQLType for User
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

    fn resolve_field(
        &self,
        info: &(),
        field_name: &str,
        args: &Arguments,
        executor: &Executor<Database>
    )
        -> ExecutionResult
    {
        // Next, we need to match the queried field name. All arms of this
        // match statement return `ExecutionResult`, which makes it hard to
        // statically verify that the type you pass on to `executor.resolve*`
        // actually matches the one that you defined in `meta()` above.
        let database = executor.context();
        match field_name {
            // Because scalars are defined with another `Context` associated
            // type, you must use resolve_with_ctx here to make the executor
            // perform automatic type conversion of its argument.
            "id" => executor.resolve_with_ctx(info, &self.id),
            "name" => executor.resolve_with_ctx(info, &self.name),

            // You pass a vector of User objects to `executor.resolve`, and it
            // will determine which fields of the sub-objects to actually
            // resolve based on the query. The executor instance keeps track
            // of its current position in the query.
            "friends" => executor.resolve(info,
                &self.friend_ids.iter()
                    .filter_map(|id| database.users.get(id))
                    .collect::<Vec<_>>()
            ),

            // We can only reach this panic in two cases; either a mismatch
            // between the defined schema in `meta()` above, or a validation
            // in this library failed because of a bug.
            //
            // In either of those two cases, the only reasonable way out is
            // to panic the thread.
            _ => panic!("Field {} not found on type User", field_name),
        }
    }
}
```

*/
pub trait GraphQLType<S = DefaultScalarValue>: Sized
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    /// The expected context type for this GraphQL type
    ///
    /// The context is threaded through query execution to all affected nodes,
    /// and can be used to hold common data, e.g. database connections or
    /// request session information.
    type Context;

    /// Type that may carry additional schema information
    ///
    /// This can be used to implement a schema that is partly dynamic,
    /// meaning that it can use information that is not known at compile time,
    /// for instance by reading it from a configuration file at start-up.
    type TypeInfo;

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

    /// Resolve the value of a single field on this type.
    ///
    /// The arguments object contain all specified arguments, with default
    /// values substituted for the ones not provided by the query.
    ///
    /// The executor can be used to drive selections into sub-objects.
    ///
    /// The default implementation panics.
    #[allow(unused_variables)]
    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field_name: &str,
        arguments: &Arguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        panic!("resolve_field must be implemented by object types");
    }

    /// Resolve this interface or union into a concrete type
    ///
    /// Try to resolve the current type into the type name provided. If the
    /// type matches, pass the instance along to `executor.resolve`.
    ///
    /// The default implementation panics.
    #[allow(unused_variables)]
    fn resolve_into_type(
        &self,
        info: &Self::TypeInfo,
        type_name: &str,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        if Self::name(info).unwrap() == type_name {
            Ok(self.resolve(info, selection_set, executor))
        } else {
            panic!("resolve_into_type must be implemented by unions and interfaces");
        }
    }

    /// Return the concrete type name for this instance/union.
    ///
    /// The default implementation panics.
    #[allow(unused_variables)]
    fn concrete_type_name(&self, context: &Self::Context, info: &Self::TypeInfo) -> String {
        panic!("concrete_type_name must be implemented by unions and interfaces");
    }

    /// Resolve the provided selection set against the current object.
    ///
    /// For non-object types, the selection set will be `None` and the value
    /// of the object should simply be returned.
    ///
    /// For objects, all fields in the selection set should be resolved.
    ///
    /// The default implementation uses `resolve_field` to resolve all fields,
    /// including those through fragment expansion, for object types. For
    /// non-object types, this method panics.
    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection_set: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> Value<S> {
        if let Some(selection_set) = selection_set {
            let mut result = Object::with_capacity(selection_set.len());
            if resolve_selection_set_into(self, info, selection_set, executor, &mut result) {
                Value::Object(result)
            } else {
                Value::null()
            }
        } else {
            panic!("resolve() must be implemented by non-object output types");
        }
    }
}

/**
This trait replaces GraphQLType`'s resolver logic with synchronous subscription
execution logic. It should be used with `GraphQLType` in order to implement
subscription GraphQL objects.

Synchronous subscription related convenience macros expand into an
implementation of this trait and `GraphQLType` for the given type.

See trait methods for more detailed explanation on how this trait works.

## Manual implementation example

The following example demonstrates how to manually implement
`GraphQLSubscriptionType` with default resolver logic (without overwriting
`resolve_into_iterator`).

Juniper's subscription macros use similar logic by default.


```rust
use juniper::{
    GraphQLType, GraphQLSubscriptionType, Value, ValuesIterator,
    FieldError, Registry, meta::MetaType,
};

#[derive(Debug)]
struct User { id: String, name: String, friend_ids: Vec<String> }

#[juniper::object]
impl User {}

struct Subscription;

// GraphQLType should be implemented in order to add `Subscription`
// to `juniper::RootNode`. In this example it is implemented manually
// to show that only `name` and `meta` methods are used, not the ones
// containing execution logic.
impl GraphQLType for Subscription {
    type Context = ();
    type TypeInfo = ();

    fn name(_: &Self::TypeInfo) -> Option<&str> {
        Some("Subscription")
    }

    fn meta<'r>(
        info: &Self::TypeInfo,
        registry: &mut Registry<'r>
    ) -> MetaType<'r>
    where juniper::DefaultScalarValue: 'r,
    {
        let fields = vec![
            registry.field_convert::<User, _, Self::Context>("users", info),
        ];
        let meta = registry.build_object_type::<Subscription>(info, &fields);
        meta.into_meta()
    }
}

impl GraphQLSubscriptionType for Subscription {
    // This function will be called every time a field is found
    fn resolve_field_into_iterator<'res>(
        &self,
        info: &Self::TypeInfo,
        field_name: &str,
        arguments: &juniper::Arguments,
        executor: std::rc::Rc<juniper::Executor<'res, Self::Context>>,
    ) -> Result<Value<ValuesIterator<'res>>, FieldError> {
        match field_name {
            "users" => {
                let users_iterator = std::iter::once(User {
                    id: "1".to_string(),
                    name: "iterator user".to_string(),
                    friend_ids: vec![
                        "2".to_string(),
                        "3".to_string(),
                        "4".to_string()
                    ]
                });
                // Each `User` that is returned from iterator should be resolved
                // as well to filter out fields that were not requested.
                // This could be done by resolving each object returned from
                // iterator as a separate query, which was executed up to
                // getting the right GraphQLObject
                let iter = users_iterator.map(move |res| {
                    // if context could be replaced...
                    juniper::IntoResolvable::into(res, executor.context())
                        .and_then(|res| match res {
                            Some((ctx, r)) => {
                                // ...resolve each `User` with current context
                                // (note that here `resolve_with_ctx` is called,
                                //  which resolves each `User` object as
                                //  separate query)
                                executor
                                    .replaced_context(ctx)
                                    .resolve_with_ctx(&(), &r)
                            }
                            // ...or return Null since `User` couldn't be
                            // resolved
                            None => Ok(Value::null()),
                        })
                        .unwrap_or_else(|_| Value::Null)
                });
                // `Value::Scalar` is returned here because we resolved
                // subscription into a single iterator (other types of values
                // can be returned as well, but they might not be supported by
                // default implementation (see methods that you're going to use
                // for more details))
                Ok(Value::Scalar(Box::new(iter)))
            },
            _ => {
                // panicking here because juniper should return field does not
                // exist error while parsing subscription query
                panic!("Field {} not found on type GraphQLSubscriptionType", &field_name);
            }
        }
    }
}
```
*/
pub trait GraphQLSubscriptionType<S = DefaultScalarValue>: GraphQLType<S> + Send + Sync
where
    S: ScalarValue + 'static,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    /// Resolve the provided selection set against the current object.
    /// This method is called by executor and should call
    /// other trait methods (if needed).
    ///
    /// This method replaces `GraphQLType::resolve`.
    ///
    /// ## Default implementation
    ///
    /// In order to resolve selection set on object types, default
    /// implementation calls `resolve_field_into_iterator` every time
    /// a field needs to be resolved and `resolve_type_into_iterator` every time
    /// a fragment needs to be resolved.
    ///
    /// For non-object types, the selection set will be `None`
    /// and default implementation will panic.
    fn resolve_into_iterator<'s, 'i, 'ss, 'e, 'res>(
        &'s self,
        info: &'i Self::TypeInfo,
        selection_set: Option<&'ss [Selection<S>]>,
        executor: &'e Executor<'e, Self::Context, S>,
    ) -> Value<ValuesIterator<'res, S>>
    where
        's: 'res,
        'i: 'res,
        'ss: 'res,
        'e: 'res,
    {
        if let Some(set) = selection_set {
            let mut obj = Object::with_capacity(set.len());
            if resolve_selection_set_into_iter(self, info, set, executor, &mut obj) {
                Value::Object(obj)
            } else {
                Value::Null
            }
        } else {
            panic!("resolve_into_iterator() must be implemented for empty selection sets");
        }
    }

    /// This method is called by Self's `resolve_into_iterator` default
    /// implementation every time any field is found in selection set.
    ///
    /// It replaces `GraphQLType::resolve_field`.
    /// Unlike `resolve_field`, which resolves each field into a single
    /// `Value<S>`, this method resolves each field into
    /// `Value<ValuesIterator<S>>`.
    ///
    /// The default implementation panics.
    fn resolve_field_into_iterator<'res>(
        &self,
        _: &Self::TypeInfo, // this subscription's type info
        _: &str,            // field's type name
        _: &Arguments<S>,   // field's arguments
        _: Rc<Executor<'res, Self::Context, S>>, // field's executor (subscription's sub-executor
                            // with current field's selection set)
    ) -> Result<Value<ValuesIterator<'res, S>>, FieldError<S>> {
        panic!("resolve_field_into_iterator must be implemented in order to resolve fields on subscriptions");
    }

    /// It is called by Self's `resolve_into_iterator` default implementation
    /// every time any fragment is found in selection set.
    ///
    /// It replaces `GraphQLType::resolve_into_type`.
    /// Unlike `resolve_into_type`, which resolves each fragment
    /// a single `Value<S>`, this method resolves each fragment into
    /// `Value<ValuesIterator<S>>`.
    ///
    /// The default implementation panics.
    fn resolve_into_type_iterator<'res>(
        &self,
        _: &Self::TypeInfo,         // this subscription's type info
        _: &str,                    // fragment's type name
        _: Option<&[Selection<S>]>, // fragment's selection set
        _: Rc<Executor<'res, Self::Context, S>>, // fragment's executor (subscription's sub-executor
                                    //              with current fragment's selection set)
    ) -> Result<Value<ValuesIterator<'res, S>>, FieldError<S>> {
        //TODO: if type is same as self call resolve_into_iterator
        // if Self::name(info).unwrap() == type_name {
        //      Ok(self.resolve_into_iterator(info, selection_set, executor))
        // } else {
        panic!("iter_resolve_into_type must be implemented in order to resolve fragments on subscriptions");
        // }
    }
}

/// Resolver logic for queries'/mutations' selection set.
/// Calls appropriate resolver method for each field or fragment found
/// and then merges returned values into `result` or pushes errors to
/// field's/fragment's sub executor.
///
/// Returns false if any errors occured and true otherwise.
pub(crate) fn resolve_selection_set_into<T, CtxT, S>(
    instance: &T,
    info: &T::TypeInfo,
    selection_set: &[Selection<S>],
    executor: &Executor<CtxT, S>,
    result: &mut Object<S>,
) -> bool
where
    T: GraphQLType<S, Context = CtxT>,
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
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
                    result.add_field(
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
                    response_name,
                    f.name.item,
                    start_pos.clone(),
                    f.selection_set.as_ref().map(|v| &v[..]),
                );

                let field_result = instance.resolve_field(
                    info,
                    f.name.item,
                    &Arguments::new(
                        f.arguments.as_ref().map(|m| {
                            m.item
                                .iter()
                                .map(|&(ref k, ref v)| {
                                    (k.item, v.item.clone().into_const(exec_vars))
                                })
                                .collect()
                        }),
                        &meta_field.arguments,
                    ),
                    &sub_exec,
                );

                match field_result {
                    Ok(Value::Null) if meta_field.field_type.is_non_null() => return false,
                    Ok(v) => merge_key_into(result, response_name, v),
                    Err(e) => {
                        sub_exec.push_error_at(e, start_pos.clone());

                        if meta_field.field_type.is_non_null() {
                            return false;
                        }

                        result.add_field(response_name, Value::null());
                    }
                }
            }
            Selection::FragmentSpread(Spanning {
                item: ref spread, ..
            }) => {
                if is_excluded(&spread.directives, executor.variables()) {
                    continue;
                }

                let fragment = &executor
                    .fragment_by_name(spread.name.item)
                    .expect("Fragment could not be found");

                if !resolve_selection_set_into(
                    instance,
                    info,
                    &fragment.selection_set[..],
                    executor,
                    result,
                ) {
                    return false;
                }
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
                    let sub_result = instance.resolve_into_type(
                        info,
                        type_condition.item,
                        Some(&fragment.selection_set[..]),
                        &sub_exec,
                    );

                    if let Ok(Value::Object(object)) = sub_result {
                        for (k, v) in object {
                            merge_key_into(result, &k, v);
                        }
                    } else if let Err(e) = sub_result {
                        sub_exec.push_error_at(e, start_pos.clone());
                    }
                } else if !resolve_selection_set_into(
                    instance,
                    info,
                    &fragment.selection_set[..],
                    &sub_exec,
                    result,
                ) {
                    return false;
                }
            }
        }
    }

    true
}

/// Resolver logic for subscriptions' selection set.
/// Calls appropriate resolver method for each field or fragment found
/// and then merges returned values into `result` or pushes errors to
/// field's/fragment's sub executor
///
/// Returns false if any errors occured and true otherwise.
pub(crate) fn resolve_selection_set_into_iter<'i, 'inf, 'ss, 'e, 'res, T, CtxT, S>(
    instance: &'i T,
    info: &'inf T::TypeInfo,
    selection_set: &'ss [Selection<S>],
    executor: &'e Executor<'e, CtxT, S>,
    result: &mut Object<ValuesIterator<'res, S>>,
) -> bool
where
    'i: 'res,
    'inf: 'res,
    'ss: 'res,
    'e: 'res,
    T: GraphQLSubscriptionType<S, Context = CtxT>,
    S: ScalarValue + 'static,
    for<'b> &'b S: ScalarRefValue<'b>,
{
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
                    result.add_field(
                        response_name,
                        Value::Scalar(Box::new(std::iter::once(Value::scalar(
                            instance.concrete_type_name(executor.context(), info),
                        )))),
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

                let sub_exec = Rc::new(executor.field_sub_executor(
                    response_name,
                    f.name.item,
                    start_pos.clone(),
                    f.selection_set.as_ref().map(|v| &v[..]),
                ));

                let sub_exec_2 = Rc::clone(&sub_exec);

                let field_result = instance.resolve_field_into_iterator(
                    info,
                    f.name.item,
                    &Arguments::new(
                        f.arguments.as_ref().map(|m| {
                            m.item
                                .iter()
                                .map(|&(ref k, ref v)| {
                                    (k.item, v.item.clone().into_const(exec_vars))
                                })
                                .collect()
                        }),
                        &meta_field.arguments,
                    ),
                    sub_exec,
                );

                match field_result {
                    Ok(Value::Null) if meta_field.field_type.is_non_null() => return false,
                    Ok(v) => {
                        merge_key_into(result, response_name, v);
                    }
                    Err(e) => {
                        sub_exec_2.push_error_at(e, start_pos.clone());

                        if meta_field.field_type.is_non_null() {
                            return false;
                        }

                        result.add_field(response_name, Value::Null);
                    }
                };
            }
            Selection::FragmentSpread(Spanning {
                item: ref spread, ..
            }) => {
                if is_excluded(&spread.directives, executor.variables()) {
                    continue;
                }

                let fragment = &executor
                    .fragment_by_name(spread.name.item)
                    .expect("Fragment could not be found");

                if !resolve_selection_set_into_iter(
                    instance,
                    info,
                    &fragment.selection_set[..],
                    executor,
                    result,
                ) {
                    return false;
                }
            }
            Selection::InlineFragment(Spanning {
                item: ref fragment,
                start: ref start_pos,
                ..
            }) => {
                if is_excluded(&fragment.directives, executor.variables()) {
                    continue;
                }

                let sub_exec = Rc::new(executor.type_sub_executor(
                    fragment.type_condition.as_ref().map(|c| c.item),
                    Some(&fragment.selection_set[..]),
                ));

                let sub_exec2 = Rc::clone(&sub_exec);

                if let Some(ref type_condition) = fragment.type_condition {
                    let sub_result = instance.resolve_into_type_iterator(
                        info,
                        type_condition.item,
                        Some(&fragment.selection_set[..]),
                        sub_exec,
                    );

                    if let Ok(Value::Object(object)) = sub_result {
                        for (k, v) in object {
                            merge_key_into(result, &k, v);
                        }
                    } else if let Err(e) = sub_result {
                        sub_exec2.push_error_at(e, start_pos.clone());
                    }
                } else {
                    if let Some(type_name) = meta_type.name() {
                        let sub_result = instance.resolve_into_type_iterator(
                            info,
                            type_name,
                            Some(&fragment.selection_set[..]),
                            sub_exec,
                        );

                        if let Ok(Value::Object(object)) = sub_result {
                            for (k, v) in object {
                                merge_key_into(result, &k, v);
                            }
                        } else if let Err(e) = sub_result {
                            sub_exec2.push_error_at(e, start_pos.clone());
                        }
                    } else {
                        return false;
                    }
                }
            }
        }
    }

    true
}

/// Checks if a field/fragment is excluded via `@include`/`@skip` GraphQL directives.
pub(super) fn is_excluded<S>(
    directives: &Option<Vec<Spanning<Directive<S>>>>,
    vars: &Variables<S>,
) -> bool
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
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
                        .for_each(|(d, s)| match d {
                            &mut Value::Object(ref mut d_obj) => {
                                if let Value::Object(s_obj) = s {
                                    merge_maps(d_obj, s_obj);
                                }
                            }
                            _ => {}
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
