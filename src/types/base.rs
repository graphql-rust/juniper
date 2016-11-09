use std::collections::HashMap;

use ast::{InputValue, Selection, Directive, FromInputValue};
use value::Value;

use schema::meta::{Argument, MetaType};
use executor::{Executor, Registry, ExecutionResult};
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
pub struct Arguments {
    args: Option<HashMap<String, InputValue>>,
}

impl Arguments {
    #[doc(hidden)]
    pub fn new(mut args: Option<HashMap<String, InputValue>>, meta_args: &Option<Vec<Argument>>) -> Arguments {
        if meta_args.is_some() && args.is_none() {
            args = Some(HashMap::new());
        }

        if let (&mut Some(ref mut args), &Some(ref meta_args)) = (&mut args, meta_args) {
            for arg in meta_args {
                if !args.contains_key(&arg.name) || args[&arg.name].is_null() {
                    if let Some(ref default_value) = arg.default_value {
                        args.insert(arg.name.clone(), default_value.clone());
                    } else {
                        args.insert(arg.name.clone(), InputValue::null());
                    }
                }
            }
        }

        Arguments {
            args: args
        }
    }

    /// Get and convert an argument into the desired type.
    ///
    /// If the argument is found, or a default argument has been provided,
    /// the `InputValue` will be converted into the type `T`.
    ///
    /// Returns `Some` if the argument is present _and_ type conversion
    /// succeeeds.
    pub fn get<T>(&self, key: &str) -> Option<T> where T: FromInputValue {
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
use juniper::{GraphQLType, Registry, FieldResult,
              Arguments, Executor, ExecutionResult};
use juniper::meta::MetaType;
# use std::collections::HashMap;

struct User { id: String, name: String, friend_ids: Vec<String>  }
struct Database { users: HashMap<String, User> }

impl GraphQLType<Database> for User {
    fn name() -> Option<&'static str> {
        Some("User")
    }

    fn meta(registry: &mut Registry<Database>) -> MetaType {
        // First, we need to define all fields and their types on this type.
        //
        // If need arguments, want to implement interfaces, or want to add
        // documentation strings, we can do it here.
        registry.build_object_type::<User>()(&[
                registry.field::<&String>("id"),
                registry.field::<&String>("name"),
                registry.field::<Vec<&User>>("friends"),
            ])
            .into_meta()
    }

    fn resolve_field(
        &self,
        field_name: &str,
        args: &Arguments,
        executor: &mut Executor<Database>
    )
        -> ExecutionResult
    {
        // Next, we need to match the queried field name. All arms of this
        // match statement return `ExecutionResult`, which makes it hard to
        // statically verify that the type you pass on to `executor.resolve`
        // actually matches the one that you defined in `meta()` above.
        let database = executor.context();
        match field_name {
            "id" => executor.resolve(&self.id),
            "name" => executor.resolve(&self.name),

            // You pass a vector of User objects to `executor.resolve`, and it
            // will determine which fields of the sub-objects to actually
            // resolve based on the query. The executor instance keeps track
            // of its current position in the query.
            "friends" => executor.resolve(
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
pub trait GraphQLType<CtxT>: Sized {
    /// The name of the GraphQL type to expose.
    ///
    /// This function will be called multiple times during schema construction.
    /// It must _not_ perform any calculation and _always_ return the same
    /// value.
    fn name() -> Option<&'static str>;

    /// The meta type representing this GraphQL type.
    fn meta(registry: &mut Registry<CtxT>) -> MetaType;

    /// Resolve the value of a single field on this type.
    ///
    /// The arguments object contain all specified arguments, with default
    /// values substituted for the ones not provided by the query.
    ///
    /// The executor can be used to drive selections into sub-objects.
    ///
    /// The default implementation panics through `unimplemented!()`.
    #[allow(unused_variables)]
    fn resolve_field(&self, field_name: &str, arguments: &Arguments, executor: &mut Executor<CtxT>)
        -> ExecutionResult
    {
        unimplemented!()
    }

    /// Resolve this interface or union into a concrete type
    ///
    /// Try to resolve the current type into the type name provided. If the
    /// type matches, pass the instance along to `executor.resolve`.
    ///
    /// The default implementation panics through `unimplemented()`.
    #[allow(unused_variables)]
    fn resolve_into_type(&self, type_name: &str, selection_set: Option<Vec<Selection>>, executor: &mut Executor<CtxT>) -> ExecutionResult {
        unimplemented!();
    }

    /// Return the concrete type name for this instance/union.
    ///
    /// The default implementation panics through `unimplemented()`.
    #[allow(unused_variables)]
    fn concrete_type_name(&self, context: &CtxT) -> String {
        unimplemented!();
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
    /// non-object types, this method panics through `unimplemented!()`.
    fn resolve(&self, selection_set: Option<Vec<Selection>>, executor: &mut Executor<CtxT>) -> Value {
        if let Some(selection_set) = selection_set {
            let mut result = HashMap::new();
            resolve_selection_set_into(self, selection_set, executor, &mut result);
            Value::object(result)
        }
        else {
            unimplemented!();
        }
    }
}

fn resolve_selection_set_into<T, CtxT>(
    instance: &T,
    selection_set: Vec<Selection>,
    executor: &mut Executor<CtxT>,
    result: &mut HashMap<String, Value>)
    where T: GraphQLType<CtxT>
{
    let meta_type = executor.schema()
        .concrete_type_by_name(T::name().expect("Resolving named type's selection set"))
        .expect("Type not found in schema");

    for selection in selection_set {
        match selection {
            Selection::Field(Spanning { item: f, start: start_pos, .. }) => {
                if is_excluded(
                        &match f.directives {
                            Some(sel) => Some(sel.iter().cloned().map(|s| s.item).collect()),
                            None => None,
                        },
                        executor.variables()) {
                    continue;
                }

                let response_name = &f.alias.as_ref().unwrap_or(&f.name).item;

                if &f.name.item == "__typename" {
                    result.insert(
                        response_name.clone(),
                        Value::string(
                            instance.concrete_type_name(executor.context())));
                    continue;
                }

                let meta_field = meta_type.field_by_name(&f.name.item)
                    .expect(&format!("Field {} not found on type {:?}", f.name.item, meta_type.name()));

                let exec_vars = executor.variables();

                let mut sub_exec = executor.sub_executor(
                    Some(response_name.clone()),
                    start_pos.clone(),
                    f.selection_set);

                let field_result = instance.resolve_field(
                    &f.name.item,
                    &Arguments::new(
                        f.arguments.map(|m|
                            m.item.into_iter().map(|(k, v)|
                                (k.item, v.item.into_const(exec_vars))).collect()),
                        &meta_field.arguments),
                    &mut sub_exec);

                match field_result {
                    Ok(v) => { result.insert(response_name.clone(), v); }
                    Err(e) => { sub_exec.push_error(e, start_pos); }
                }
            },
            Selection::FragmentSpread(Spanning { item: spread, .. }) => {
                if is_excluded(
                        &match spread.directives {
                            Some(sel) => Some(sel.iter().cloned().map(|s| s.item).collect()),
                            None => None,
                        },
                        executor.variables()) {
                    continue;
                }

                let fragment = &executor.fragment_by_name(&spread.name.item)
                    .expect("Fragment could not be found");

                resolve_selection_set_into(
                    instance, fragment.selection_set.clone(), executor, result);
            },
            Selection::InlineFragment(Spanning { item: fragment, start: start_pos, .. }) => {
                if is_excluded(
                        &match fragment.directives {
                            Some(sel) => Some(sel.iter().cloned().map(|s| s.item).collect()),
                            None => None
                        },
                        executor.variables()) {
                    continue;
                }

                let mut sub_exec = executor.sub_executor(
                    None,
                    start_pos.clone(),
                    Some(fragment.selection_set.clone()));

                if let Some(type_condition) = fragment.type_condition {
                    let sub_result = instance.resolve_into_type(
                        &type_condition.item,
                        Some(fragment.selection_set.clone()),
                        &mut sub_exec);

                    if let Ok(Value::Object(mut hash_map)) = sub_result {
                        for (k, v) in hash_map.drain() {
                            result.insert(k, v);
                        }
                    }
                    else if let Err(e) = sub_result {
                         sub_exec.push_error(e, start_pos);
                    }
                }
                else {
                    resolve_selection_set_into(
                        instance,
                        fragment.selection_set.clone(),
                        &mut sub_exec,
                        result);
                }
            },
        }
    }
}

fn is_excluded(directives: &Option<Vec<Directive>>, vars: &HashMap<String, InputValue>) -> bool {
    if let Some(ref directives) = *directives {
        for directive in directives {
            let condition: bool = directive.arguments.iter()
                .flat_map(|m| m.item.get("if"))
                .flat_map(|v| v.item.clone().into_const(vars).convert())
                .next().unwrap();

            if directive.name.item == "skip" && condition {
                return true
            }
            else if directive.name.item == "include" && !condition {
                return true
            }
        }

        false
    }
    else {
        false
    }
}
