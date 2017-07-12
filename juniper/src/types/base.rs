use std::collections::HashMap;
use std::collections::hash_map::Entry;

use ast::{Directive, FromInputValue, InputValue, Selection};
use executor::Variables;
use value::Value;

use schema::meta::{Argument, MetaType};
use executor::{ExecutionResult, Executor, Registry};
use parser::Spanning;

/// GraphQL type kind
///
/// The GraphQL specification defines a number of type kinds - the meta type
/// of a type.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum TypeKind {
    /// ## Scalar types
    ///
    /// Scalar types appear as the leaf nodes of GraphQL queries. Strings,
    /// numbers, and booleans are the built in types, and while it's possible
    /// to define your own, it's relatively uncommon.
    Scalar,

    /// ## Object types
    ///
    /// The most common type to be implemented by users. Objects have fields
    /// and can implement interfaces.
    Object,

    /// ## Interface types
    ///
    /// Interface types are used to represent overlapping fields between
    /// multiple types, and can be queried for their concrete type.
    Interface,

    /// ## Union types
    ///
    /// Unions are similar to interfaces but can not contain any fields on
    /// their own.
    Union,

    /// ## Enum types
    ///
    /// Like scalars, enum types appear as the leaf nodes of GraphQL queries.
    Enum,

    /// ## Input objects
    ///
    /// Represents complex values provided in queries _into_ the system.
    InputObject,

    /// ## List types
    ///
    /// Represent lists of other types. This library provides implementations
    /// for vectors and slices, but other Rust types can be extended to serve
    /// as GraphQL lists.
    List,

    /// ## Non-null types
    ///
    /// In GraphQL, nullable types are the default. By putting a `!` after a
    /// type, it becomes non-nullable.
    NonNull,
}

/// Field argument container
pub struct Arguments<'a> {
    args: Option<HashMap<&'a str, InputValue>>,
}

impl<'a> Arguments<'a> {
    #[doc(hidden)]
    pub fn new(
        mut args: Option<HashMap<&'a str, InputValue>>,
        meta_args: &'a Option<Vec<Argument>>,
    ) -> Arguments<'a> {
        if meta_args.is_some() && args.is_none() {
            args = Some(HashMap::new());
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

        Arguments { args: args }
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
        T: FromInputValue,
    {
        match self.args {
            Some(ref args) => match args.get(key) {
                Some(v) => Some(v.convert().unwrap()),
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
              Arguments, Executor, ExecutionResult};
use juniper::meta::MetaType;
# use std::collections::HashMap;

struct User { id: String, name: String, friend_ids: Vec<String>  }
struct Database { users: HashMap<String, User> }

impl Context for Database {}

impl GraphQLType for User {
    type Context = Database;
    type TypeInfo = ();

    fn name(_: &()) -> Option<&'static str> {
        Some("User")
    }

    fn meta<'r>(_: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
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
pub trait GraphQLType: Sized {
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
    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r>;

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
        arguments: &Arguments,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
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
        selection_set: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
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
    fn concrete_type_name(&self, context: &Self::Context) -> String {
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
        selection_set: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> Value {
        if let Some(selection_set) = selection_set {
            let mut result = HashMap::new();
            resolve_selection_set_into(self, info, selection_set, executor, &mut result);
            Value::object(result)
        } else {
            panic!("resolve() must be implemented by non-object output types");
        }
    }
}

fn resolve_selection_set_into<T, CtxT>(
    instance: &T,
    info: &T::TypeInfo,
    selection_set: &[Selection],
    executor: &Executor<CtxT>,
    result: &mut HashMap<String, Value>,
) where
    T: GraphQLType<Context = CtxT>,
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

                let response_name = &f.alias.as_ref().unwrap_or(&f.name).item;

                if f.name.item == "__typename" {
                    result.insert(
                        (*response_name).to_owned(),
                        Value::string(instance.concrete_type_name(executor.context())),
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

                let sub_exec = executor.sub_executor(
                    Some(response_name),
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
                    Ok(v) => merge_key_into(result, response_name, v),
                    Err(e) => {
                        sub_exec.push_error(e, start_pos.clone());
                        result.insert((*response_name).to_owned(), Value::null());
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

                resolve_selection_set_into(
                    instance,
                    info,
                    &fragment.selection_set[..],
                    executor,
                    result,
                );
            }
            Selection::InlineFragment(Spanning {
                item: ref fragment,
                start: ref start_pos,
                ..
            }) => {
                if is_excluded(&fragment.directives, executor.variables()) {
                    continue;
                }

                let sub_exec = executor
                    .sub_executor(None, start_pos.clone(), Some(&fragment.selection_set[..]));

                if let Some(ref type_condition) = fragment.type_condition {
                    let sub_result = instance.resolve_into_type(
                        info,
                        type_condition.item,
                        Some(&fragment.selection_set[..]),
                        &sub_exec,
                    );

                    if let Ok(Value::Object(mut hash_map)) = sub_result {
                        for (k, v) in hash_map.drain() {
                            result.insert(k, v);
                        }
                    } else if let Err(e) = sub_result {
                        sub_exec.push_error(e, start_pos.clone());
                    }
                } else {
                    resolve_selection_set_into(
                        instance,
                        info,
                        &fragment.selection_set[..],
                        &sub_exec,
                        result,
                    );
                }
            }
        }
    }
}

fn is_excluded(directives: &Option<Vec<Spanning<Directive>>>, vars: &Variables) -> bool {
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

            if (directive.name.item == "skip" && condition) ||
                (directive.name.item == "include" && !condition)
            {
                return true;
            }
        }
    }
    false
}

fn merge_key_into(result: &mut HashMap<String, Value>, response_name: &str, value: Value) {
    match result.entry(response_name.to_owned()) {
        Entry::Occupied(mut e) => match (e.get_mut().as_mut_object_value(), value) {
            (Some(dest_obj), Value::Object(src_obj)) => {
                merge_maps(dest_obj, src_obj);
            }
            _ => {}
        },
        Entry::Vacant(e) => {
            e.insert(value);
        }
    }
}

fn merge_maps(dest: &mut HashMap<String, Value>, src: HashMap<String, Value>) {
    for (key, value) in src {
        if dest.contains_key(&key) {
            merge_key_into(dest, &key, value);
        } else {
            dest.insert(key, value);
        }
    }
}
