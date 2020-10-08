use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::HashMap,
    fmt::{Debug, Display},
    sync::{Arc, RwLock},
};

use fnv::FnvHashMap;
use futures::Stream;

use crate::{
    ast::{
        Definition, Document, Fragment, FromInputValue, InputValue, Operation, OperationType,
        Selection, ToInputValue, Type,
    },
    parser::{SourcePosition, Spanning},
    schema::{
        meta::{
            Argument, DeprecationStatus, EnumMeta, EnumValue, Field, InputObjectMeta,
            InterfaceMeta, ListMeta, MetaType, NullableMeta, ObjectMeta, PlaceholderMeta,
            ScalarMeta, UnionMeta,
        },
        model::{RootNode, SchemaType, TypeType},
    },
    types::{
        async_await::{GraphQLTypeAsync, GraphQLValueAsync},
        base::{GraphQLType, GraphQLValue},
        name::Name,
        subscriptions::{GraphQLSubscriptionType, GraphQLSubscriptionValue},
    },
    value::{ParseScalarValue, Value},
    GraphQLError,
};

pub use self::{
    look_ahead::{
        Applies, ChildSelection, ConcreteLookAheadSelection, LookAheadArgument, LookAheadMethods,
        LookAheadSelection, LookAheadValue,
    },
    owned_executor::OwnedExecutor,
};

mod look_ahead;
mod owned_executor;

/// A type registry used to build schemas
///
/// The registry gathers metadata for all types in a schema. It provides
/// convenience methods to convert types implementing the `GraphQLType` trait
/// into `Type` instances and automatically registers them.
pub struct Registry<'r> {
    /// Currently registered types
    pub types: FnvHashMap<Name, MetaType<'r>>,
}

#[derive(Clone)]
pub enum FieldPath<'a> {
    Root(SourcePosition),
    Field(&'a str, SourcePosition, Arc<FieldPath<'a>>),
}

/// Query execution engine
///
/// The executor helps drive the query execution in a schema. It keeps track
/// of the current field stack, context, variables, and errors.
pub struct Executor<'r, 'a, CtxT>
where
    CtxT: 'a,
{
    fragments: &'r HashMap<&'a str, Fragment<'a>>,
    variables: &'r Variables,
    current_selection_set: Option<&'r [Selection<'a>]>,
    parent_selection_set: Option<&'r [Selection<'a>]>,
    current_type: TypeType<'a>,
    schema: &'a SchemaType<'a>,
    context: &'a CtxT,
    errors: &'r RwLock<Vec<ExecutionError>>,
    field_path: Arc<FieldPath<'a>>,
}

/// Error type for errors that occur during query execution
///
/// All execution errors contain the source position in the query of the field
/// that failed to resolve. It also contains the field stack.
#[derive(Debug, PartialEq)]
pub struct ExecutionError {
    location: SourcePosition,
    path: Vec<String>,
    error: FieldError,
}

impl Eq for ExecutionError where Self: PartialEq {}

impl ExecutionError {
    /// Construct a new execution error occuring at the beginning of the query
    pub fn at_origin(error: FieldError) -> ExecutionError {
        ExecutionError {
            location: SourcePosition::new_origin(),
            path: Vec::new(),
            error,
        }
    }
}

impl PartialOrd for ExecutionError
where
    Self: PartialEq,
{
    fn partial_cmp(&self, other: &ExecutionError) -> Option<Ordering> {
        (&self.location, &self.path, &self.error.message).partial_cmp(&(
            &other.location,
            &other.path,
            &other.error.message,
        ))
    }
}

impl Ord for ExecutionError
where
    Self: Eq,
{
    fn cmp(&self, other: &ExecutionError) -> Ordering {
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
pub struct FieldError {
    message: String,
    extensions: Value,
}

impl<T: Display> From<T> for FieldError {
    fn from(e: T) -> FieldError {
        FieldError {
            message: format!("{}", e),
            extensions: Value::null(),
        }
    }
}

impl FieldError {
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
    pub fn new<T: Display>(e: T, extensions: Value) -> FieldError {
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
    pub fn extensions(&self) -> &Value {
        &self.extensions
    }
}

/// The result of resolving the value of a field of type `T`
pub type FieldResult<T> = Result<T, FieldError>;

/// The result of resolving an unspecified field
pub type ExecutionResult = Result<Value, FieldError>;

/// Boxed `Stream` yielding `Result<Value, ExecutionError>`
pub type ValuesStream<'a> =
    std::pin::Pin<Box<dyn Stream<Item = Result<Value, ExecutionError>> + Send + 'a>>;

/// The map of variables used for substitution during query execution
pub type Variables = HashMap<String, InputValue>;

/// Custom error handling trait to enable Error types other than `FieldError` to be specified
/// as return value.
///
/// Any custom error type should implement this trait to convert it to `FieldError`.
pub trait IntoFieldError {
    #[doc(hidden)]
    fn into_field_error(self) -> FieldError;
}

impl IntoFieldError for FieldError {
    fn into_field_error(self) -> FieldError {
        self
    }
}

#[doc(hidden)]
pub trait IntoResolvable<'a, T, C>
where
    T: GraphQLValue,
{
    type Type;

    #[doc(hidden)]
    fn into(self, ctx: &'a C) -> FieldResult<Option<(&'a T::Context, T)>>;
}

impl<'a, T, C> IntoResolvable<'a, T, C> for T
where
    T: GraphQLValue,
    T::Context: FromContext<C>,
{
    type Type = T;

    fn into(self, ctx: &'a C) -> FieldResult<Option<(&'a T::Context, T)>> {
        Ok(Some((FromContext::from(ctx), self)))
    }
}

impl<'a, T, C, E: IntoFieldError> IntoResolvable<'a, T, C> for Result<T, E>
where
    T: GraphQLValue,
    T::Context: FromContext<C>,
{
    type Type = T;

    fn into(self, ctx: &'a C) -> FieldResult<Option<(&'a T::Context, T)>> {
        self.map(|v: T| Some((<T::Context as FromContext<C>>::from(ctx), v)))
            .map_err(IntoFieldError::into_field_error)
    }
}

impl<'a, T, C> IntoResolvable<'a, T, C> for (&'a T::Context, T)
where
    T: GraphQLValue,
{
    type Type = T;

    fn into(self, _: &'a C) -> FieldResult<Option<(&'a T::Context, T)>> {
        Ok(Some(self))
    }
}

impl<'a, T, C> IntoResolvable<'a, Option<T>, C> for Option<(&'a T::Context, T)>
where
    T: GraphQLValue,
{
    type Type = T;

    fn into(self, _: &'a C) -> FieldResult<Option<(&'a T::Context, Option<T>)>> {
        Ok(self.map(|(ctx, v)| (ctx, Some(v))))
    }
}

impl<'a, T, C> IntoResolvable<'a, T, C> for FieldResult<(&'a T::Context, T)>
where
    T: GraphQLValue,
{
    type Type = T;

    fn into(self, _: &'a C) -> FieldResult<Option<(&'a T::Context, T)>> {
        self.map(Some)
    }
}

impl<'a, T, C> IntoResolvable<'a, Option<T>, C> for FieldResult<Option<(&'a T::Context, T)>>
where
    T: GraphQLValue,
{
    type Type = T;

    fn into(self, _: &'a C) -> FieldResult<Option<(&'a T::Context, Option<T>)>> {
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

impl<'r, 'a, CtxT> Executor<'r, 'a, CtxT> {
    /// Resolve a single arbitrary value into a stream of [`Value`]s.
    /// If a field fails to resolve, pushes error to `Executor`
    /// and returns `Value::Null`.
    pub async fn resolve_into_stream<'i, 'v, 'res, T>(
        &'r self,
        info: &'i T::TypeInfo,
        value: &'v T,
    ) -> Value<ValuesStream<'res>>
    where
        'i: 'res,
        'v: 'res,
        'a: 'res,
        T: GraphQLSubscriptionValue<Context = CtxT> + ?Sized,
        T::TypeInfo: Sync,
        CtxT: Sync,
    {
        self.subscribe(info, value).await.unwrap_or_else(|e| {
            self.push_error(e);
            Value::Null
        })
    }

    /// Resolve a single arbitrary value into a stream of [`Value`]s.
    /// Calls `resolve_into_stream` on `T`.
    pub async fn subscribe<'s, 't, 'res, T>(
        &'r self,
        info: &'t T::TypeInfo,
        value: &'t T,
    ) -> Result<Value<ValuesStream<'res>>, FieldError>
    where
        't: 'res,
        'a: 'res,
        T: GraphQLSubscriptionValue<Context = CtxT> + ?Sized,
        T::TypeInfo: Sync,
        CtxT: Sync,
    {
        value.resolve_into_stream(info, self).await
    }

    /// Resolve a single arbitrary value, mapping the context to a new type
    pub fn resolve_with_ctx<NewCtxT, T>(&self, info: &T::TypeInfo, value: &T) -> ExecutionResult
    where
        NewCtxT: FromContext<CtxT>,
        T: GraphQLValue<Context = NewCtxT> + ?Sized,
    {
        self.replaced_context(<NewCtxT as FromContext<CtxT>>::from(self.context))
            .resolve(info, value)
    }

    /// Resolve a single arbitrary value into an `ExecutionResult`
    pub fn resolve<T>(&self, info: &T::TypeInfo, value: &T) -> ExecutionResult
    where
        T: GraphQLValue<Context = CtxT> + ?Sized,
    {
        value.resolve(info, self.current_selection_set, self)
    }

    /// Resolve a single arbitrary value into an `ExecutionResult`
    pub async fn resolve_async<T>(&self, info: &T::TypeInfo, value: &T) -> ExecutionResult
    where
        T: GraphQLValueAsync<Context = CtxT> + ?Sized,
        T::TypeInfo: Sync,
        CtxT: Sync,
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
    ) -> ExecutionResult
    where
        T: GraphQLValueAsync<Context = NewCtxT> + ?Sized,
        T::TypeInfo: Sync,
        NewCtxT: FromContext<CtxT> + Sync,
    {
        let e = self.replaced_context(<NewCtxT as FromContext<CtxT>>::from(self.context));
        e.resolve_async(info, value).await
    }

    /// Resolve a single arbitrary value into a return value
    ///
    /// If the field fails to resolve, `null` will be returned.
    pub fn resolve_into_value<T>(&self, info: &T::TypeInfo, value: &T) -> Value
    where
        T: GraphQLValue<Context = CtxT>,
    {
        self.resolve(info, value).unwrap_or_else(|e| {
            self.push_error(e);
            Value::null()
        })
    }

    /// Resolve a single arbitrary value into a return value
    ///
    /// If the field fails to resolve, `null` will be returned.
    pub async fn resolve_into_value_async<T>(&self, info: &T::TypeInfo, value: &T) -> Value
    where
        T: GraphQLValueAsync<Context = CtxT> + ?Sized,
        T::TypeInfo: Sync,
        CtxT: Sync,
    {
        self.resolve_async(info, value).await.unwrap_or_else(|e| {
            self.push_error(e);
            Value::null()
        })
    }

    /// Derive a new executor by replacing the context
    ///
    /// This can be used to connect different types, e.g. from different Rust
    /// libraries, that require different context types.
    pub fn replaced_context<'b, NewCtxT>(&'b self, ctx: &'b NewCtxT) -> Executor<'b, 'b, NewCtxT> {
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
    pub fn field_sub_executor<'s>(
        &'s self,
        field_alias: &'a str,
        field_name: &'s str,
        location: SourcePosition,
        selection_set: Option<&'s [Selection<'a>]>,
    ) -> Executor<'s, 'a, CtxT> {
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
            field_path: Arc::new(FieldPath::Field(
                field_alias,
                location,
                Arc::clone(&self.field_path),
            )),
        }
    }

    #[doc(hidden)]
    pub fn type_sub_executor<'s>(
        &'s self,
        type_name: Option<&'s str>,
        selection_set: Option<&'s [Selection<'a>]>,
    ) -> Executor<'s, 'a, CtxT> {
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

    /// `Executor`'s current selection set
    pub(crate) fn current_selection_set(&self) -> Option<&[Selection<'a>]> {
        self.current_selection_set
    }

    /// Access the current context
    ///
    /// You usually provide the context when calling the top-level `execute`
    /// function, or using the context factory in the Iron integration.
    pub fn context(&self) -> &'r CtxT {
        self.context
    }

    /// The currently executing schema
    pub fn schema(&self) -> &'a SchemaType {
        self.schema
    }

    #[doc(hidden)]
    pub fn current_type(&self) -> &TypeType<'a> {
        &self.current_type
    }

    #[doc(hidden)]
    pub fn variables(&self) -> &'r Variables {
        self.variables
    }

    #[doc(hidden)]
    pub fn fragment_by_name<'s>(&'s self, name: &str) -> Option<&'s Fragment<'a>> {
        self.fragments.get(name)
    }

    /// The current location of the executor
    pub fn location(&self) -> &SourcePosition {
        self.field_path.location()
    }

    /// Add an error to the execution engine at the current executor location
    pub fn push_error(&self, error: FieldError) {
        self.push_error_at(error, *self.location());
    }

    /// Add an error to the execution engine at a specific location
    pub fn push_error_at(&self, error: FieldError, location: SourcePosition) {
        let mut path = Vec::new();
        self.field_path.construct_path(&mut path);

        let mut errors = self.errors.write().unwrap();

        errors.push(ExecutionError {
            location,
            path,
            error,
        });
    }

    /// Returns new [`ExecutionError`] at current location
    pub fn new_error(&self, error: FieldError) -> ExecutionError {
        let mut path = Vec::new();
        self.field_path.construct_path(&mut path);

        ExecutionError {
            location: *self.location(),
            path,
            error,
        }
    }

    /// Construct a lookahead selection for the current selection.
    ///
    /// This allows seeing the whole selection and perform operations
    /// affecting the children.
    pub fn look_ahead(&'a self) -> LookAheadSelection<'a> {
        let field_name = match *self.field_path {
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
                                        &s,
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

    /// Create new `OwnedExecutor` and clone all current data
    /// (except for errors) there
    ///
    /// New empty vector is created for `errors` because
    /// existing errors won't be needed to be accessed by user
    /// in OwnedExecutor as existing errors will be returned in
    /// `execute_query`/`execute_mutation`/`resolve_into_stream`/etc.
    pub fn as_owned_executor(&self) -> OwnedExecutor<'a, CtxT> {
        OwnedExecutor {
            fragments: self.fragments.clone(),
            variables: self.variables.clone(),
            current_selection_set: self.current_selection_set.map(|x| x.to_vec()),
            parent_selection_set: self.parent_selection_set.map(|x| x.to_vec()),
            current_type: self.current_type.clone(),
            schema: self.schema,
            context: self.context,
            errors: RwLock::new(vec![]),
            field_path: Arc::clone(&self.field_path),
        }
    }
}

impl<'a> FieldPath<'a> {
    fn construct_path(&self, acc: &mut Vec<String>) {
        match self {
            FieldPath::Root(_) => (),
            FieldPath::Field(name, _, parent) => {
                parent.construct_path(acc);
                acc.push((*name).to_owned());
            }
        }
    }

    fn location(&self) -> &SourcePosition {
        match *self {
            FieldPath::Root(ref pos) | FieldPath::Field(_, ref pos, _) => pos,
        }
    }
}

impl ExecutionError {
    #[doc(hidden)]
    pub fn new(location: SourcePosition, path: &[&str], error: FieldError) -> ExecutionError {
        ExecutionError {
            location,
            path: path.iter().map(|s| (*s).to_owned()).collect(),
            error,
        }
    }

    /// The error message
    pub fn error(&self) -> &FieldError {
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

/// Create new `Executor` and start query/mutation execution.
/// Returns `IsSubscription` error if subscription is passed.
pub fn execute_validated_query<'a, 'b, QueryT, MutationT, SubscriptionT>(
    document: &'b Document,
    operation: &'b Spanning<Operation>,
    root_node: &RootNode<QueryT, MutationT, SubscriptionT>,
    variables: &Variables,
    context: &QueryT::Context,
) -> Result<(Value, Vec<ExecutionError>), GraphQLError<'a>>
where
    QueryT: GraphQLType,
    MutationT: GraphQLType<Context = QueryT::Context>,
    SubscriptionT: GraphQLType<Context = QueryT::Context>,
{
    if operation.item.operation_type == OperationType::Subscription {
        return Err(GraphQLError::IsSubscription);
    }

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
            .collect::<HashMap<String, InputValue>>()
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
                .map(|f| (f.item.name.item, f.item.clone()))
                .collect(),
            variables: final_vars,
            current_selection_set: Some(&operation.item.selection_set[..]),
            parent_selection_set: None,
            current_type: root_type,
            schema: &root_node.schema,
            context,
            errors: &errors,
            field_path: Arc::new(FieldPath::Root(operation.start)),
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

/// Create new `Executor` and start asynchronous query execution.
/// Returns `IsSubscription` error if subscription is passed.
pub async fn execute_validated_query_async<'a, 'b, QueryT, MutationT, SubscriptionT>(
    document: &'b Document<'a>,
    operation: &'b Spanning<Operation<'_>>,
    root_node: &RootNode<'a, QueryT, MutationT, SubscriptionT>,
    variables: &Variables,
    context: &QueryT::Context,
) -> Result<(Value, Vec<ExecutionError>), GraphQLError<'a>>
where
    QueryT: GraphQLTypeAsync,
    QueryT::TypeInfo: Sync,
    QueryT::Context: Sync,
    MutationT: GraphQLTypeAsync<Context = QueryT::Context>,
    MutationT::TypeInfo: Sync,
    SubscriptionT: GraphQLType<Context = QueryT::Context> + Sync,
    SubscriptionT::TypeInfo: Sync,
{
    if operation.item.operation_type == OperationType::Subscription {
        return Err(GraphQLError::IsSubscription);
    }

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
            .collect::<HashMap<String, InputValue>>()
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
                .map(|f| (f.item.name.item, f.item.clone()))
                .collect(),
            variables: final_vars,
            current_selection_set: Some(&operation.item.selection_set[..]),
            parent_selection_set: None,
            current_type: root_type,
            schema: &root_node.schema,
            context,
            errors: &errors,
            field_path: Arc::new(FieldPath::Root(operation.start)),
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

pub fn get_operation<'b, 'd, 'e>(
    document: &'b Document<'d>,
    operation_name: Option<&str>,
) -> Result<&'b Spanning<Operation<'d>>, GraphQLError<'e>>
where
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
    Ok(op)
}

/// Initialize new `Executor` and start resolving subscription into stream
/// asynchronously.
/// Returns `NotSubscription` error if query or mutation is passed
pub async fn resolve_validated_subscription<
    'r,
    'exec_ref,
    'd,
    'op,
    QueryT,
    MutationT,
    SubscriptionT,
>(
    document: &Document<'d>,
    operation: &Spanning<Operation<'op>>,
    root_node: &'r RootNode<'r, QueryT, MutationT, SubscriptionT>,
    variables: &Variables,
    context: &'r QueryT::Context,
) -> Result<(Value<ValuesStream<'r>>, Vec<ExecutionError>), GraphQLError<'r>>
where
    'r: 'exec_ref,
    'd: 'r,
    'op: 'd,
    QueryT: GraphQLTypeAsync,
    QueryT::TypeInfo: Sync,
    QueryT::Context: Sync + 'r,
    MutationT: GraphQLTypeAsync<Context = QueryT::Context>,
    MutationT::TypeInfo: Sync,
    SubscriptionT: GraphQLSubscriptionType<Context = QueryT::Context>,
    SubscriptionT::TypeInfo: Sync,
{
    if operation.item.operation_type != OperationType::Subscription {
        return Err(GraphQLError::NotSubscription);
    }

    let mut fragments = vec![];
    for def in document.iter() {
        if let Definition::Fragment(f) = def {
            fragments.push(f)
        }
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
            .collect::<HashMap<String, InputValue>>()
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
            OperationType::Subscription => root_node
                .schema
                .subscription_type()
                .expect("No subscription type found"),
            _ => unreachable!(),
        };

        let executor: Executor<'_, 'r, _> = Executor {
            fragments: &fragments
                .iter()
                .map(|f| (f.item.name.item, f.item.clone()))
                .collect(),
            variables: final_vars,
            current_selection_set: Some(&operation.item.selection_set[..]),
            parent_selection_set: None,
            current_type: root_type,
            schema: &root_node.schema,
            context,
            errors: &errors,
            field_path: Arc::new(FieldPath::Root(operation.start)),
        };

        value = match operation.item.operation_type {
            OperationType::Subscription => {
                executor
                    .resolve_into_stream(&root_node.subscription_info, &root_node.subscription_type)
                    .await
            }
            _ => unreachable!(),
        };
    }

    let mut errors = errors.into_inner().unwrap();
    errors.sort();

    Ok((value, errors))
}

impl<'r> Registry<'r> {
    /// Construct a new registry
    pub fn new(types: FnvHashMap<Name, MetaType<'r>>) -> Registry<'r> {
        Registry { types }
    }

    /// Get the `Type` instance for a given GraphQL type
    ///
    /// If the registry hasn't seen a type with this name before, it will
    /// construct its metadata and store it.
    pub fn get_type<T>(&mut self, info: &T::TypeInfo) -> Type<'r>
    where
        T: GraphQLType + ?Sized,
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
    pub fn field<T>(&mut self, name: &str, info: &T::TypeInfo) -> Field<'r>
    where
        T: GraphQLType + ?Sized,
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
    pub fn field_convert<'a, T: IntoResolvable<'a, I, C>, I, C>(
        &mut self,
        name: &str,
        info: &I::TypeInfo,
    ) -> Field<'r>
    where
        I: GraphQLType,
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
    pub fn arg<T>(&mut self, name: &str, info: &T::TypeInfo) -> Argument<'r>
    where
        T: GraphQLType + FromInputValue + ?Sized,
    {
        Argument::new(name, self.get_type::<T>(info))
    }

    /// Create an argument with a default value
    ///
    /// When called with type `T`, the actual argument will be given the type
    /// `Option<T>`.
    pub fn arg_with_default<T>(&mut self, name: &str, value: &T, info: &T::TypeInfo) -> Argument<'r>
    where
        T: GraphQLType + ToInputValue + FromInputValue + ?Sized,
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
    pub fn build_scalar_type<T>(&mut self, info: &T::TypeInfo) -> ScalarMeta<'r>
    where
        T: FromInputValue + GraphQLType + ParseScalarValue + ?Sized + 'r,
    {
        let name = T::name(info).expect("Scalar types must be named. Implement name()");
        ScalarMeta::new::<T>(Cow::Owned(name.to_string()))
    }

    /// Create a list meta type
    pub fn build_list_type<T: GraphQLType + ?Sized>(&mut self, info: &T::TypeInfo) -> ListMeta<'r> {
        let of_type = self.get_type::<T>(info);
        ListMeta::new(of_type)
    }

    /// Create a nullable meta type
    pub fn build_nullable_type<T: GraphQLType + ?Sized>(
        &mut self,
        info: &T::TypeInfo,
    ) -> NullableMeta<'r> {
        let of_type = self.get_type::<T>(info);
        NullableMeta::new(of_type)
    }

    /// Create an object meta type
    ///
    /// To prevent infinite recursion by enforcing ordering, this returns a
    /// function that needs to be called with the list of fields on the object.
    pub fn build_object_type<T>(
        &mut self,
        info: &T::TypeInfo,
        fields: &[Field<'r>],
    ) -> ObjectMeta<'r>
    where
        T: GraphQLType + ?Sized,
    {
        let name = T::name(info).expect("Object types must be named. Implement name()");

        let mut v = fields.to_vec();
        v.push(self.field::<String>("__typename", &()));
        ObjectMeta::new(Cow::Owned(name.to_string()), &v)
    }

    /// Create an enum meta type
    pub fn build_enum_type<T>(&mut self, info: &T::TypeInfo, values: &[EnumValue]) -> EnumMeta<'r>
    where
        T: FromInputValue + GraphQLType + ?Sized,
    {
        let name = T::name(info).expect("Enum types must be named. Implement name()");

        EnumMeta::new::<T>(Cow::Owned(name.to_string()), values)
    }

    /// Create an interface meta type,
    /// by providing a type info object.
    pub fn build_interface_type<T>(
        &mut self,
        info: &T::TypeInfo,
        fields: &[Field<'r>],
    ) -> InterfaceMeta<'r>
    where
        T: GraphQLType + ?Sized,
    {
        let name = T::name(info).expect("Interface types must be named. Implement name()");

        let mut v = fields.to_vec();
        v.push(self.field::<String>("__typename", &()));
        InterfaceMeta::new(Cow::Owned(name.to_string()), &v)
    }

    /// Create a union meta type
    pub fn build_union_type<T>(&mut self, info: &T::TypeInfo, types: &[Type<'r>]) -> UnionMeta<'r>
    where
        T: GraphQLType + ?Sized,
    {
        let name = T::name(info).expect("Union types must be named. Implement name()");

        UnionMeta::new(Cow::Owned(name.to_string()), types)
    }

    /// Create an input object meta type
    pub fn build_input_object_type<T>(
        &mut self,
        info: &T::TypeInfo,
        args: &[Argument<'r>],
    ) -> InputObjectMeta<'r>
    where
        T: FromInputValue + GraphQLType + ?Sized,
    {
        let name = T::name(info).expect("Input object types must be named. Implement name()");

        InputObjectMeta::new::<T>(Cow::Owned(name.to_string()), args)
    }
}
