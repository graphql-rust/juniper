use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::HashMap,
    fmt::{Debug, Display},
    sync::RwLock,
};

use fnv::FnvHashMap;

use crate::{
    ast::{
        Definition, Document, Fragment, FromInputValue, InputValue, Operation, OperationType,
        Selection, ToInputValue, Type,
    },
    parser::SourcePosition,
    value::Value,
    GraphQLError,
};

use crate::schema::{
    meta::{
        Argument, DeprecationStatus, EnumMeta, EnumValue, Field, InputObjectMeta, InterfaceMeta,
        ListMeta, MetaType, NullableMeta, ObjectMeta, PlaceholderMeta, ScalarMeta, UnionMeta,
    },
    model::{RootNode, SchemaType, TypeType},
};

use crate::{
    types::{base::GraphQLType, name::Name},
    value::{DefaultScalarValue, ParseScalarValue, ScalarValue},
};

mod look_ahead;

pub use self::look_ahead::{
    Applies, ChildSelection, ConcreteLookAheadSelection, LookAheadArgument, LookAheadMethods,
    LookAheadSelection, LookAheadValue,
};
use crate::parser::Spanning;

/// A type registry used to build schemas
///
/// The registry gathers metadata for all types in a schema. It provides
/// convenience methods to convert types implementing the `GraphQLType` trait
/// into `Type` instances and automatically registers them.
pub struct Registry<'r, S = DefaultScalarValue> {
    /// Currently registered types
    pub types: FnvHashMap<Name, MetaType<'r, S>>,
}

#[derive(Clone)]
pub enum FieldPath<'a> {
    Root(SourcePosition),
    Field(&'a str, SourcePosition, &'a FieldPath<'a>),
}

/// Query execution engine
///
/// The executor helps drive the query execution in a schema. It keeps track
/// of the current field stack, context, variables, and errors.
pub struct Executor<'a, CtxT, S = DefaultScalarValue>
where
    CtxT: 'a,
    S: 'a,
{
    fragments: &'a HashMap<&'a str, &'a Fragment<'a, S>>,
    variables: &'a Variables<S>,
    current_selection_set: Option<&'a [Selection<'a, S>]>,
    parent_selection_set: Option<&'a [Selection<'a, S>]>,
    current_type: TypeType<'a, S>,
    schema: &'a SchemaType<'a, S>,
    context: &'a CtxT,
    errors: &'a RwLock<Vec<ExecutionError<S>>>,
    field_path: FieldPath<'a>,
}

/// Error type for errors that occur during query execution
///
/// All execution errors contain the source position in the query of the field
/// that failed to resolve. It also contains the field stack.
#[derive(Debug, PartialEq)]
pub struct ExecutionError<S> {
    location: SourcePosition,
    path: Vec<String>,
    error: FieldError<S>,
}

impl<S> Eq for ExecutionError<S> where Self: PartialEq {}

impl<S> ExecutionError<S> {
    /// Construct a new execution error occuring at the beginning of the query
    pub fn at_origin(error: FieldError<S>) -> ExecutionError<S> {
        ExecutionError {
            location: SourcePosition::new_origin(),
            path: Vec::new(),
            error,
        }
    }
}

impl<S> PartialOrd for ExecutionError<S>
where
    Self: PartialEq,
{
    fn partial_cmp(&self, other: &ExecutionError<S>) -> Option<Ordering> {
        (&self.location, &self.path, &self.error.message).partial_cmp(&(
            &other.location,
            &other.path,
            &other.error.message,
        ))
    }
}

impl<S> Ord for ExecutionError<S>
where
    Self: Eq,
{
    fn cmp(&self, other: &ExecutionError<S>) -> Ordering {
        (&self.location, &self.path, &self.error.message).cmp(&(
            &other.location,
            &other.path,
            &other.error.message,
        ))
    }
}

/// Error type for errors that occur during field resolution
///
/// Field errors are represented by a human-readable error message and an
/// optional `Value` structure containing additional information.
///
/// They can be converted to from any type that implements `std::fmt::Display`,
/// which makes error chaining with the `?` operator a breeze:
///
/// ```rust
/// # use juniper::{FieldError, ScalarValue};
/// fn get_string(data: Vec<u8>) -> Result<String, FieldError>
/// {
///     let s = String::from_utf8(data)?;
///     Ok(s)
/// }
/// ```
#[derive(Debug, PartialEq)]
pub struct FieldError<S = DefaultScalarValue> {
    message: String,
    extensions: Value<S>,
}

impl<T: Display, S> From<T> for FieldError<S>
where
    S: crate::value::ScalarValue,
{
    fn from(e: T) -> FieldError<S> {
        FieldError {
            message: format!("{}", e),
            extensions: Value::null(),
        }
    }
}

impl<S> FieldError<S> {
    /// Construct a new error with additional data
    ///
    /// You can use the `graphql_value!` macro to construct an error:
    ///
    /// ```rust
    /// # extern crate juniper;
    /// use juniper::FieldError;
    /// # use juniper::DefaultScalarValue;
    /// use juniper::graphql_value;
    ///
    /// # fn sample() {
    /// # let _: FieldError<DefaultScalarValue> =
    /// FieldError::new(
    ///     "Could not open connection to the database",
    ///     graphql_value!({ "internal_error": "Connection refused" })
    /// );
    /// # }
    /// # fn main() { }
    /// ```
    ///
    /// The `extensions` parameter will be added to the `"extensions"` field of the error
    /// object in the JSON response:
    ///
    /// ```json
    /// {
    ///   "errors": [
    ///     "message": "Could not open connection to the database",
    ///     "locations": [{"line": 2, "column": 4}],
    ///     "extensions": {
    ///       "internal_error": "Connection refused"
    ///     }
    ///   ]
    /// }
    /// ```
    ///
    /// If the argument is `Value::null()`, no extra data will be included.
    pub fn new<T: Display>(e: T, extensions: Value<S>) -> FieldError<S> {
        FieldError {
            message: format!("{}", e),
            extensions,
        }
    }

    #[doc(hidden)]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[doc(hidden)]
    pub fn extensions(&self) -> &Value<S> {
        &self.extensions
    }
}

/// The result of resolving the value of a field of type `T`
pub type FieldResult<T, S = DefaultScalarValue> = Result<T, FieldError<S>>;

/// The result of resolving an unspecified field
pub type ExecutionResult<S = DefaultScalarValue> = Result<Value<S>, FieldError<S>>;

/// The map of variables used for substitution during query execution
pub type Variables<S = DefaultScalarValue> = HashMap<String, InputValue<S>>;

/// Custom error handling trait to enable Error types other than `FieldError` to be specified
/// as return value.
///
/// Any custom error type should implement this trait to convert it to `FieldError`.
pub trait IntoFieldError<S = DefaultScalarValue> {
    #[doc(hidden)]
    fn into_field_error(self) -> FieldError<S>;
}

impl<S> IntoFieldError<S> for FieldError<S> {
    fn into_field_error(self) -> FieldError<S> {
        self
    }
}

#[doc(hidden)]
pub trait IntoResolvable<'a, S, T: GraphQLType<S>, C>: Sized
where
    S: ScalarValue,
{
    #[doc(hidden)]
    fn into(self, ctx: &'a C) -> FieldResult<Option<(&'a T::Context, T)>, S>;
}

impl<'a, S, T, C> IntoResolvable<'a, S, T, C> for T
where
    T: GraphQLType<S>,
    S: ScalarValue,
    T::Context: FromContext<C>,
{
    fn into(self, ctx: &'a C) -> FieldResult<Option<(&'a T::Context, T)>, S> {
        Ok(Some((FromContext::from(ctx), self)))
    }
}

impl<'a, S, T, C, E: IntoFieldError<S>> IntoResolvable<'a, S, T, C> for Result<T, E>
where
    S: ScalarValue,
    T: GraphQLType<S>,
    T::Context: FromContext<C>,
{
    fn into(self, ctx: &'a C) -> FieldResult<Option<(&'a T::Context, T)>, S> {
        self.map(|v: T| Some((<T::Context as FromContext<C>>::from(ctx), v)))
            .map_err(IntoFieldError::into_field_error)
    }
}

impl<'a, S, T, C> IntoResolvable<'a, S, T, C> for (&'a T::Context, T)
where
    S: ScalarValue,
    T: GraphQLType<S>,
{
    fn into(self, _: &'a C) -> FieldResult<Option<(&'a T::Context, T)>, S> {
        Ok(Some(self))
    }
}

impl<'a, S, T, C> IntoResolvable<'a, S, Option<T>, C> for Option<(&'a T::Context, T)>
where
    S: ScalarValue,
    T: GraphQLType<S>,
{
    fn into(self, _: &'a C) -> FieldResult<Option<(&'a T::Context, Option<T>)>, S> {
        Ok(self.map(|(ctx, v)| (ctx, Some(v))))
    }
}

impl<'a, S, T, C> IntoResolvable<'a, S, T, C> for FieldResult<(&'a T::Context, T), S>
where
    S: ScalarValue,
    T: GraphQLType<S>,
{
    fn into(self, _: &'a C) -> FieldResult<Option<(&'a T::Context, T)>, S> {
        self.map(Some)
    }
}

impl<'a, S, T, C> IntoResolvable<'a, S, Option<T>, C>
    for FieldResult<Option<(&'a T::Context, T)>, S>
where
    S: ScalarValue,
    T: GraphQLType<S>,
{
    fn into(self, _: &'a C) -> FieldResult<Option<(&'a T::Context, Option<T>)>, S> {
        self.map(|o| o.map(|(ctx, v)| (ctx, Some(v))))
    }
}

/// Conversion trait for context types
///
/// Used to support different context types for different parts of an
/// application. By making each `GraphQL` type only aware of as much
/// context as it needs to, isolation and robustness can be
/// improved. Implement this trait if you have contexts that can
/// generally be converted between each other.
///
/// The empty tuple `()` can be converted into from any context type,
/// making it suitable for `GraphQL` that don't need _any_ context to
/// work, e.g. scalars or enums.
pub trait FromContext<T> {
    /// Perform the conversion
    fn from(value: &T) -> &Self;
}

/// Marker trait for types that can act as context objects for `GraphQL` types.
pub trait Context {}

impl<'a, C: Context> Context for &'a C {}

static NULL_CONTEXT: () = ();

impl<T> FromContext<T> for () {
    fn from(_: &T) -> &Self {
        &NULL_CONTEXT
    }
}

impl<T> FromContext<T> for T
where
    T: Context,
{
    fn from(value: &T) -> &Self {
        value
    }
}

impl<'a, CtxT, S> Executor<'a, CtxT, S>
where
    S: ScalarValue,
{
    /// Resolve a single arbitrary value, mapping the context to a new type
    pub fn resolve_with_ctx<NewCtxT, T>(&self, info: &T::TypeInfo, value: &T) -> ExecutionResult<S>
    where
        NewCtxT: FromContext<CtxT>,
        T: GraphQLType<S, Context = NewCtxT>,
    {
        self.replaced_context(<NewCtxT as FromContext<CtxT>>::from(self.context))
            .resolve(info, value)
    }

    /// Resolve a single arbitrary value into an `ExecutionResult`
    pub fn resolve<T>(&self, info: &T::TypeInfo, value: &T) -> ExecutionResult<S>
    where
        T: GraphQLType<S, Context = CtxT>,
    {
        value.resolve(info, self.current_selection_set, self)
    }

    /// Resolve a single arbitrary value into an `ExecutionResult`
    pub async fn resolve_async<T>(&self, info: &T::TypeInfo, value: &T) -> ExecutionResult<S>
    where
        T: crate::GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
        T::TypeInfo: Send + Sync,
        CtxT: Send + Sync,
        S: Send + Sync,
    {
        value
            .resolve_async(info, self.current_selection_set, self)
            .await
    }

    /// Resolve a single arbitrary value, mapping the context to a new type
    pub async fn resolve_with_ctx_async<NewCtxT, T>(
        &self,
        info: &T::TypeInfo,
        value: &T,
    ) -> ExecutionResult<S>
    where
        T: crate::GraphQLTypeAsync<S, Context = NewCtxT> + Send + Sync,
        T::TypeInfo: Send + Sync,
        S: Send + Sync,
        NewCtxT: FromContext<CtxT> + Send + Sync,
    {
        let e = self.replaced_context(<NewCtxT as FromContext<CtxT>>::from(self.context));
        e.resolve_async(info, value).await
    }

    /// Resolve a single arbitrary value into a return value
    ///
    /// If the field fails to resolve, `null` will be returned.
    pub fn resolve_into_value<T>(&self, info: &T::TypeInfo, value: &T) -> Value<S>
    where
        T: GraphQLType<S, Context = CtxT>,
    {
        match self.resolve(info, value) {
            Ok(v) => v,
            Err(e) => {
                self.push_error(e);
                Value::null()
            }
        }
    }

    /// Resolve a single arbitrary value into a return value
    ///
    /// If the field fails to resolve, `null` will be returned.
    pub async fn resolve_into_value_async<T>(&self, info: &T::TypeInfo, value: &T) -> Value<S>
    where
        T: crate::GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
        T::TypeInfo: Send + Sync,
        CtxT: Send + Sync,
        S: Send + Sync,
    {
        match self.resolve_async(info, value).await {
            Ok(v) => v,
            Err(e) => {
                self.push_error(e);
                Value::null()
            }
        }
    }

    /// Derive a new executor by replacing the context
    ///
    /// This can be used to connect different types, e.g. from different Rust
    /// libraries, that require different context types.
    pub fn replaced_context<'b, NewCtxT>(&'b self, ctx: &'b NewCtxT) -> Executor<'b, NewCtxT, S> {
        Executor {
            fragments: self.fragments,
            variables: self.variables,
            current_selection_set: self.current_selection_set,
            parent_selection_set: self.parent_selection_set,
            current_type: self.current_type.clone(),
            schema: self.schema,
            context: ctx,
            errors: self.errors,
            field_path: self.field_path.clone(),
        }
    }

    #[doc(hidden)]
    pub fn field_sub_executor(
        &self,
        field_alias: &'a str,
        field_name: &'a str,
        location: SourcePosition,
        selection_set: Option<&'a [Selection<S>]>,
    ) -> Executor<CtxT, S> {
        Executor {
            fragments: self.fragments,
            variables: self.variables,
            current_selection_set: selection_set,
            parent_selection_set: self.current_selection_set,
            current_type: self.schema.make_type(
                &self
                    .current_type
                    .innermost_concrete()
                    .field_by_name(field_name)
                    .expect("Field not found on inner type")
                    .field_type,
            ),
            schema: self.schema,
            context: self.context,
            errors: self.errors,
            field_path: FieldPath::Field(field_alias, location, &self.field_path),
        }
    }

    #[doc(hidden)]
    pub fn type_sub_executor(
        &self,
        type_name: Option<&'a str>,
        selection_set: Option<&'a [Selection<S>]>,
    ) -> Executor<CtxT, S> {
        Executor {
            fragments: self.fragments,
            variables: self.variables,
            current_selection_set: selection_set,
            parent_selection_set: self.current_selection_set,
            current_type: match type_name {
                Some(type_name) => self.schema.type_by_name(type_name).expect("Type not found"),
                None => self.current_type.clone(),
            },
            schema: self.schema,
            context: self.context,
            errors: self.errors,
            field_path: self.field_path.clone(),
        }
    }

    /// Access the current context
    ///
    /// You usually provide the context when calling the top-level `execute`
    /// function, or using the context factory in the Iron integration.
    pub fn context(&self) -> &'a CtxT {
        self.context
    }

    /// The currently executing schema
    pub fn schema(&self) -> &'a SchemaType<S> {
        self.schema
    }

    #[doc(hidden)]
    pub fn current_type(&self) -> &TypeType<'a, S> {
        &self.current_type
    }

    #[doc(hidden)]
    pub fn variables(&self) -> &'a Variables<S> {
        self.variables
    }

    #[doc(hidden)]
    pub fn fragment_by_name(&'a self, name: &str) -> Option<&'a Fragment<'a, S>> {
        self.fragments.get(name).cloned()
    }

    /// The current location of the executor
    pub fn location(&self) -> &SourcePosition {
        self.field_path.location()
    }

    /// Add an error to the execution engine at the current executor location
    pub fn push_error(&self, error: FieldError<S>) {
        self.push_error_at(error, self.location().clone());
    }

    /// Add an error to the execution engine at a specific location
    pub fn push_error_at(&self, error: FieldError<S>, location: SourcePosition) {
        let mut path = Vec::new();
        self.field_path.construct_path(&mut path);

        let mut errors = self.errors.write().unwrap();

        errors.push(ExecutionError {
            location,
            path,
            error,
        });
    }

    /// Construct a lookahead selection for the current selection.
    ///
    /// This allows seeing the whole selection and perform operations
    /// affecting the children.
    pub fn look_ahead(&'a self) -> LookAheadSelection<'a, S> {
        let field_name = match self.field_path {
            FieldPath::Field(x, ..) => x,
            FieldPath::Root(_) => unreachable!(),
        };
        self.parent_selection_set
            .map(|p| {
                let found_field = p.iter().find(|&x| {
                    match *x {
                        Selection::Field(ref field) => {
                            let field = &field.item;
                            // TODO: support excludes.
                            let name = field.name.item;
                            let alias = field.alias.as_ref().map(|a| a.item);
                            alias.unwrap_or(name) == field_name
                        }
                        _ => false,
                    }
                });
                if let Some(p) = found_field {
                    LookAheadSelection::build_from_selection(&p, self.variables, self.fragments)
                } else {
                    None
                }
            })
            .filter(|s| s.is_some())
            .unwrap_or_else(|| {
                Some(LookAheadSelection {
                    name: self.current_type.innermost_concrete().name().unwrap_or(""),
                    alias: None,
                    arguments: Vec::new(),
                    children: self
                        .current_selection_set
                        .map(|s| {
                            s.iter()
                                .map(|s| ChildSelection {
                                    inner: LookAheadSelection::build_from_selection(
                                        s,
                                        self.variables,
                                        self.fragments,
                                    )
                                    .expect("a child selection"),
                                    applies_for: Applies::All,
                                })
                                .collect()
                        })
                        .unwrap_or_else(Vec::new),
                })
            })
            .unwrap_or_default()
    }
}

impl<'a> FieldPath<'a> {
    fn construct_path(&self, acc: &mut Vec<String>) {
        match *self {
            FieldPath::Root(_) => (),
            FieldPath::Field(name, _, parent) => {
                parent.construct_path(acc);
                acc.push(name.to_owned());
            }
        }
    }

    fn location(&self) -> &SourcePosition {
        match *self {
            FieldPath::Root(ref pos) | FieldPath::Field(_, ref pos, _) => pos,
        }
    }
}

impl<S> ExecutionError<S> {
    #[doc(hidden)]
    pub fn new(location: SourcePosition, path: &[&str], error: FieldError<S>) -> ExecutionError<S> {
        ExecutionError {
            location,
            path: path.iter().map(|s| (*s).to_owned()).collect(),
            error,
        }
    }

    /// The error message
    pub fn error(&self) -> &FieldError<S> {
        &self.error
    }

    /// The source location _in the query_ of the field that failed to resolve
    pub fn location(&self) -> &SourcePosition {
        &self.location
    }

    /// The path of fields leading to the field that generated this error
    pub fn path(&self) -> &[String] {
        &self.path
    }
}

pub fn execute_validated_query<'a, 'b, QueryT, MutationT, CtxT, S>(
    document: &'b Document<S>,
    operation: &'b Spanning<Operation<S>>,
    root_node: &RootNode<QueryT, MutationT, S>,
    variables: &Variables<S>,
    context: &CtxT,
) -> Result<(Value<S>, Vec<ExecutionError<S>>), GraphQLError<'a>>
where
    S: ScalarValue,
    QueryT: GraphQLType<S, Context = CtxT>,
    MutationT: GraphQLType<S, Context = CtxT>,
{
    let mut fragments = vec![];
    for def in document.iter() {
        if let Definition::Fragment(f) = def {
            fragments.push(f)
        };
    }

    let default_variable_values = operation.item.variable_definitions.as_ref().map(|defs| {
        defs.item
            .items
            .iter()
            .filter_map(|&(ref name, ref def)| {
                def.default_value
                    .as_ref()
                    .map(|i| (name.item.to_owned(), i.item.clone()))
            })
            .collect::<HashMap<String, InputValue<S>>>()
    });

    let errors = RwLock::new(Vec::new());
    let value;

    {
        let mut all_vars;
        let mut final_vars = variables;

        if let Some(defaults) = default_variable_values {
            all_vars = variables.clone();

            for (name, value) in defaults {
                all_vars.entry(name).or_insert(value);
            }

            final_vars = &all_vars;
        }

        let root_type = match operation.item.operation_type {
            OperationType::Query => root_node.schema.query_type(),
            OperationType::Mutation => root_node
                .schema
                .mutation_type()
                .expect("No mutation type found"),
            OperationType::Subscription => unreachable!(),
        };

        let executor = Executor {
            fragments: &fragments
                .iter()
                .map(|f| (f.item.name.item, &f.item))
                .collect(),
            variables: final_vars,
            current_selection_set: Some(&operation.item.selection_set[..]),
            parent_selection_set: None,
            current_type: root_type,
            schema: &root_node.schema,
            context,
            errors: &errors,
            field_path: FieldPath::Root(operation.start),
        };

        value = match operation.item.operation_type {
            OperationType::Query => executor.resolve_into_value(&root_node.query_info, &root_node),
            OperationType::Mutation => {
                executor.resolve_into_value(&root_node.mutation_info, &root_node.mutation_type)
            }
            OperationType::Subscription => unreachable!(),
        };
    }

    let mut errors = errors.into_inner().unwrap();
    errors.sort();

    Ok((value, errors))
}

pub async fn execute_validated_query_async<'a, 'b, QueryT, MutationT, CtxT, S>(
    document: &'b Document<'a, S>,
    operation: &'b Spanning<Operation<'_, S>>,
    root_node: &RootNode<'a, QueryT, MutationT, S>,
    variables: &Variables<S>,
    context: &CtxT,
) -> Result<(Value<S>, Vec<ExecutionError<S>>), GraphQLError<'a>>
where
    S: ScalarValue + Send + Sync,
    QueryT: crate::GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
    QueryT::TypeInfo: Send + Sync,
    MutationT: crate::GraphQLTypeAsync<S, Context = CtxT> + Send + Sync,
    MutationT::TypeInfo: Send + Sync,
    CtxT: Send + Sync,
{
    let mut fragments = vec![];
    for def in document.iter() {
        if let Definition::Fragment(f) = def {
            fragments.push(f)
        };
    }

    let default_variable_values = operation.item.variable_definitions.as_ref().map(|defs| {
        defs.item
            .items
            .iter()
            .filter_map(|&(ref name, ref def)| {
                def.default_value
                    .as_ref()
                    .map(|i| (name.item.to_owned(), i.item.clone()))
            })
            .collect::<HashMap<String, InputValue<S>>>()
    });

    let errors = RwLock::new(Vec::new());
    let value;

    {
        let mut all_vars;
        let mut final_vars = variables;

        if let Some(defaults) = default_variable_values {
            all_vars = variables.clone();

            for (name, value) in defaults {
                all_vars.entry(name).or_insert(value);
            }

            final_vars = &all_vars;
        }

        let root_type = match operation.item.operation_type {
            OperationType::Query => root_node.schema.query_type(),
            OperationType::Mutation => root_node
                .schema
                .mutation_type()
                .expect("No mutation type found"),
            OperationType::Subscription => unreachable!(),
        };

        let executor = Executor {
            fragments: &fragments
                .iter()
                .map(|f| (f.item.name.item, &f.item))
                .collect(),
            variables: final_vars,
            current_selection_set: Some(&operation.item.selection_set[..]),
            parent_selection_set: None,
            current_type: root_type,
            schema: &root_node.schema,
            context,
            errors: &errors,
            field_path: FieldPath::Root(operation.start),
        };

        value = match operation.item.operation_type {
            OperationType::Query => {
                executor
                    .resolve_into_value_async(&root_node.query_info, &root_node)
                    .await
            }
            OperationType::Mutation => {
                executor
                    .resolve_into_value_async(&root_node.mutation_info, &root_node.mutation_type)
                    .await
            }
            OperationType::Subscription => unreachable!(),
        };
    }

    let mut errors = errors.into_inner().unwrap();
    errors.sort();

    Ok((value, errors))
}

pub fn get_operation<'a, 'b, S>(
    document: &'b Document<'b, S>,
    operation_name: Option<&str>,
) -> Result<&'b Spanning<Operation<'b, S>>, GraphQLError<'a>>
where
    S: ScalarValue,
{
    let mut operation = None;
    for def in document {
        if let Definition::Operation(op) = def {
            if operation_name.is_none() && operation.is_some() {
                return Err(GraphQLError::MultipleOperationsProvided);
            }

            let move_op =
                operation_name.is_none() || op.item.name.as_ref().map(|s| s.item) == operation_name;

            if move_op {
                operation = Some(op);
            }
        };
    }
    let op = match operation {
        Some(op) => op,
        None => return Err(GraphQLError::UnknownOperationName),
    };
    if op.item.operation_type == OperationType::Subscription {
        return Err(GraphQLError::IsSubscription);
    }
    Ok(op)
}

impl<'r, S> Registry<'r, S>
where
    S: ScalarValue + 'r,
{
    /// Construct a new registry
    pub fn new(types: FnvHashMap<Name, MetaType<'r, S>>) -> Registry<'r, S> {
        Registry { types }
    }

    /// Get the `Type` instance for a given GraphQL type
    ///
    /// If the registry hasn't seen a type with this name before, it will
    /// construct its metadata and store it.
    pub fn get_type<T>(&mut self, info: &T::TypeInfo) -> Type<'r>
    where
        T: GraphQLType<S>,
    {
        if let Some(name) = T::name(info) {
            let validated_name = name.parse::<Name>().unwrap();
            if !self.types.contains_key(name) {
                self.insert_placeholder(
                    validated_name.clone(),
                    Type::NonNullNamed(Cow::Owned(name.to_string())),
                );
                let meta = T::meta(info, self);
                self.types.insert(validated_name, meta);
            }
            self.types[name].as_type()
        } else {
            T::meta(info, self).as_type()
        }
    }

    /// Create a field with the provided name
    pub fn field<T>(&mut self, name: &str, info: &T::TypeInfo) -> Field<'r, S>
    where
        T: GraphQLType<S>,
    {
        Field {
            name: name.to_owned(),
            description: None,
            arguments: None,
            field_type: self.get_type::<T>(info),
            deprecation_status: DeprecationStatus::Current,
        }
    }

    #[doc(hidden)]
    pub fn field_convert<'a, T: IntoResolvable<'a, S, I, C>, I, C>(
        &mut self,
        name: &str,
        info: &I::TypeInfo,
    ) -> Field<'r, S>
    where
        I: GraphQLType<S>,
    {
        Field {
            name: name.to_owned(),
            description: None,
            arguments: None,
            field_type: self.get_type::<I>(info),
            deprecation_status: DeprecationStatus::Current,
        }
    }

    /// Create an argument with the provided name
    pub fn arg<T>(&mut self, name: &str, info: &T::TypeInfo) -> Argument<'r, S>
    where
        T: GraphQLType<S> + FromInputValue<S>,
    {
        Argument::new(name, self.get_type::<T>(info))
    }

    /// Create an argument with a default value
    ///
    /// When called with type `T`, the actual argument will be given the type
    /// `Option<T>`.
    pub fn arg_with_default<T>(
        &mut self,
        name: &str,
        value: &T,
        info: &T::TypeInfo,
    ) -> Argument<'r, S>
    where
        T: GraphQLType<S> + ToInputValue<S> + FromInputValue<S>,
    {
        Argument::new(name, self.get_type::<Option<T>>(info)).default_value(value.to_input_value())
    }

    fn insert_placeholder(&mut self, name: Name, of_type: Type<'r>) {
        self.types
            .entry(name)
            .or_insert(MetaType::Placeholder(PlaceholderMeta { of_type }));
    }

    /// Create a scalar meta type
    ///
    /// This expects the type to implement `FromInputValue`.
    pub fn build_scalar_type<T>(&mut self, info: &T::TypeInfo) -> ScalarMeta<'r, S>
    where
        T: FromInputValue<S> + GraphQLType<S> + ParseScalarValue<S> + 'r,
    {
        let name = T::name(info).expect("Scalar types must be named. Implement name()");
        ScalarMeta::new::<T>(Cow::Owned(name.to_string()))
    }

    /// Create a list meta type
    pub fn build_list_type<T: GraphQLType<S>>(&mut self, info: &T::TypeInfo) -> ListMeta<'r> {
        let of_type = self.get_type::<T>(info);
        ListMeta::new(of_type)
    }

    /// Create a nullable meta type
    pub fn build_nullable_type<T: GraphQLType<S>>(
        &mut self,
        info: &T::TypeInfo,
    ) -> NullableMeta<'r> {
        let of_type = self.get_type::<T>(info);
        NullableMeta::new(of_type)
    }

    /// Create an object meta type builder
    ///
    /// To prevent infinite recursion by enforcing ordering, this returns a
    /// function that needs to be called with the list of fields on the object.
    pub fn build_object_type<T>(
        &mut self,
        info: &T::TypeInfo,
        fields: &[Field<'r, S>],
    ) -> ObjectMeta<'r, S>
    where
        T: GraphQLType<S>,
    {
        let name = T::name(info).expect("Object types must be named. Implement name()");

        let mut v = fields.to_vec();
        v.push(self.field::<String>("__typename", &()));
        ObjectMeta::new(Cow::Owned(name.to_string()), &v)
    }

    /// Create an enum meta type
    pub fn build_enum_type<T>(
        &mut self,
        info: &T::TypeInfo,
        values: &[EnumValue],
    ) -> EnumMeta<'r, S>
    where
        T: FromInputValue<S> + GraphQLType<S>,
    {
        let name = T::name(info).expect("Enum types must be named. Implement name()");

        EnumMeta::new::<T>(Cow::Owned(name.to_string()), values)
    }

    /// Create an interface meta type builder,
    /// by providing a type info object.
    pub fn build_interface_type<T>(
        &mut self,
        info: &T::TypeInfo,
        fields: &[Field<'r, S>],
    ) -> InterfaceMeta<'r, S>
    where
        T: GraphQLType<S>,
    {
        let name = T::name(info).expect("Interface types must be named. Implement name()");

        let mut v = fields.to_vec();
        v.push(self.field::<String>("__typename", &()));
        InterfaceMeta::new(Cow::Owned(name.to_string()), &v)
    }

    /// Create a union meta type builder
    pub fn build_union_type<T>(&mut self, info: &T::TypeInfo, types: &[Type<'r>]) -> UnionMeta<'r>
    where
        T: GraphQLType<S>,
    {
        let name = T::name(info).expect("Union types must be named. Implement name()");

        UnionMeta::new(Cow::Owned(name.to_string()), types)
    }

    /// Create an input object meta type builder
    pub fn build_input_object_type<T>(
        &mut self,
        info: &T::TypeInfo,
        args: &[Argument<'r, S>],
    ) -> InputObjectMeta<'r, S>
    where
        T: FromInputValue<S> + GraphQLType<S>,
    {
        let name = T::name(info).expect("Input object types must be named. Implement name()");

        InputObjectMeta::new::<T>(Cow::Owned(name.to_string()), args)
    }
}
