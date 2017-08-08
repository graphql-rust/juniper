use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::RwLock;

use GraphQLError;
use ast::{Definition, Document, Fragment, FromInputValue, InputValue, OperationType, Selection,
          ToInputValue, Type};
use value::Value;
use parser::SourcePosition;

use schema::meta::{Argument, EnumMeta, EnumValue, Field, InputObjectMeta, InterfaceMeta, ListMeta,
                   MetaType, NullableMeta, ObjectMeta, PlaceholderMeta, ScalarMeta, UnionMeta};
use schema::model::{RootNode, SchemaType};

use types::base::GraphQLType;
use types::name::Name;

/// A type registry used to build schemas
///
/// The registry gathers metadata for all types in a schema. It provides
/// convenience methods to convert types implementing the `GraphQLType` trait
/// into `Type` instances and automatically registers them.
pub struct Registry<'r> {
    /// Currently registered types
    pub types: HashMap<Name, MetaType<'r>>,
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
pub struct Executor<'a, CtxT>
where
    CtxT: 'a,
{
    fragments: &'a HashMap<&'a str, &'a Fragment<'a>>,
    variables: &'a Variables,
    current_selection_set: Option<&'a [Selection<'a>]>,
    schema: &'a SchemaType<'a>,
    context: &'a CtxT,
    errors: &'a RwLock<Vec<ExecutionError>>,
    field_path: FieldPath<'a>,
}

/// Error type for errors that occur during query execution
///
/// All execution errors contain the source position in the query of the field
/// that failed to resolve. It also contains the field stack.
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct ExecutionError {
    location: SourcePosition,
    path: Vec<String>,
    message: String,
}

/// The result of resolving the value of a field of type `T`
pub type FieldResult<T> = Result<T, String>;

/// The result of resolving an unspecified field
pub type ExecutionResult = Result<Value, String>;

/// The map of variables used for substitution during query execution
pub type Variables = HashMap<String, InputValue>;

#[doc(hidden)]
pub trait IntoResolvable<'a, T: GraphQLType, C>: Sized {
    #[doc(hidden)]
    fn into(self, ctx: &'a C) -> FieldResult<Option<(&'a T::Context, T)>>;
}

impl<'a, T: GraphQLType, C> IntoResolvable<'a, T, C> for T
where
    T::Context: FromContext<C>,
{
    fn into(self, ctx: &'a C) -> FieldResult<Option<(&'a T::Context, T)>> {
        Ok(Some((FromContext::from(ctx), self)))
    }
}

impl<'a, T: GraphQLType, C> IntoResolvable<'a, T, C> for FieldResult<T>
where
    T::Context: FromContext<C>,
{
    fn into(self, ctx: &'a C) -> FieldResult<Option<(&'a T::Context, T)>> {
        self.map(|v| Some((FromContext::from(ctx), v)))
    }
}

impl<'a, T: GraphQLType, C> IntoResolvable<'a, T, C> for (&'a T::Context, T) {
    fn into(self, _: &'a C) -> FieldResult<Option<(&'a T::Context, T)>> {
        Ok(Some(self))
    }
}

impl<'a, T: GraphQLType, C> IntoResolvable<'a, T, C> for Option<(&'a T::Context, T)> {
    fn into(self, _: &'a C) -> FieldResult<Option<(&'a T::Context, T)>> {
        Ok(self)
    }
}

impl<'a, T: GraphQLType, C> IntoResolvable<'a, T, C> for FieldResult<(&'a T::Context, T)> {
    fn into(self, _: &'a C) -> FieldResult<Option<(&'a T::Context, T)>> {
        self.map(Some)
    }
}

impl<'a, T: GraphQLType, C> IntoResolvable<'a, T, C> for FieldResult<Option<(&'a T::Context, T)>> {
    fn into(self, _: &'a C) -> FieldResult<Option<(&'a T::Context, T)>> {
        self
    }
}

/// Conversion trait for context types
///
/// Used to support different context types for different parts of an
/// application. By making each GraphQL type only aware of as much
/// context as it needs to, isolation and robustness can be
/// improved. Implement this trait if you have contexts that can
/// generally be converted between each other.
///
/// The empty tuple `()` can be converted into from any context type,
/// making it suitable for GraphQL that don't need _any_ context to
/// work, e.g. scalars or enums.
pub trait FromContext<T> {
    /// Perform the conversion
    fn from(value: &T) -> &Self;
}

/// Marker trait for types that can act as context objects for GraphQL types.
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

impl<'a, CtxT> Executor<'a, CtxT> {
    /// Resolve a single arbitrary value, mapping the context to a new type
    pub fn resolve_with_ctx<NewCtxT, T: GraphQLType<Context = NewCtxT>>(
        &self,
        info: &T::TypeInfo,
        value: &T,
    ) -> ExecutionResult
    where
        NewCtxT: FromContext<CtxT>,
    {
        self.replaced_context(<NewCtxT as FromContext<CtxT>>::from(self.context))
            .resolve(info, value)
    }

    /// Resolve a single arbitrary value into an `ExecutionResult`
    pub fn resolve<T: GraphQLType<Context = CtxT>>(
        &self,
        info: &T::TypeInfo,
        value: &T,
    ) -> ExecutionResult {
        Ok(value.resolve(info, self.current_selection_set, self))
    }

    /// Resolve a single arbitrary value into a return value
    ///
    /// If the field fails to resolve, `null` will be returned.
    pub fn resolve_into_value<T: GraphQLType<Context = CtxT>>(
        &self,
        info: &T::TypeInfo,
        value: &T,
    ) -> Value {
        match self.resolve(info, value) {
            Ok(v) => v,
            Err(e) => {
                let position = self.field_path.location().clone();
                self.push_error(e, position);
                Value::null()
            }
        }
    }

    /// Derive a new executor by replacing the context
    ///
    /// This can be used to connect different types, e.g. from different Rust
    /// libraries, that require different context types.
    pub fn replaced_context<'b, NewCtxT>(&'b self, ctx: &'b NewCtxT) -> Executor<'b, NewCtxT> {
        Executor {
            fragments: self.fragments,
            variables: self.variables,
            current_selection_set: self.current_selection_set,
            schema: self.schema,
            context: ctx,
            errors: self.errors,
            field_path: self.field_path.clone(),
        }
    }

    #[doc(hidden)]
    pub fn sub_executor(
        &self,
        field_name: Option<&'a str>,
        location: SourcePosition,
        selection_set: Option<&'a [Selection]>,
    ) -> Executor<CtxT> {
        Executor {
            fragments: self.fragments,
            variables: self.variables,
            current_selection_set: selection_set,
            schema: self.schema,
            context: self.context,
            errors: self.errors,
            field_path: match field_name {
                Some(name) => FieldPath::Field(name, location, &self.field_path),
                None => self.field_path.clone(),
            },
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
    pub fn schema(&self) -> &'a SchemaType {
        self.schema
    }

    #[doc(hidden)]
    pub fn variables(&self) -> &'a Variables {
        self.variables
    }

    #[doc(hidden)]
    pub fn fragment_by_name(&self, name: &str) -> Option<&'a Fragment> {
        self.fragments.get(name).map(|f| *f)
    }

    /// Add an error to the execution engine
    pub fn push_error(&self, error: String, location: SourcePosition) {
        let mut path = Vec::new();
        self.field_path.construct_path(&mut path);

        let mut errors = self.errors.write().unwrap();

        errors.push(ExecutionError {
            location: location,
            path: path,
            message: error,
        });
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

impl ExecutionError {
    #[doc(hidden)]
    pub fn new(location: SourcePosition, path: &[&str], message: &str) -> ExecutionError {
        ExecutionError {
            location: location,
            path: path.iter().map(|s| (*s).to_owned()).collect(),
            message: message.to_owned(),
        }
    }

    /// The error message
    pub fn message(&self) -> &str {
        &self.message
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

pub fn execute_validated_query<'a, QueryT, MutationT, CtxT>(
    document: Document,
    operation_name: Option<&str>,
    root_node: &RootNode<QueryT, MutationT>,
    variables: &Variables,
    context: &CtxT,
) -> Result<(Value, Vec<ExecutionError>), GraphQLError<'a>>
where
    QueryT: GraphQLType<Context = CtxT>,
    MutationT: GraphQLType<Context = CtxT>,
{
    let mut fragments = vec![];
    let mut operation = None;

    for def in document {
        match def {
            Definition::Operation(op) => {
                if operation_name.is_none() && operation.is_some() {
                    return Err(GraphQLError::MultipleOperationsProvided);
                }

                let move_op = operation_name.is_none() ||
                    op.item.name.as_ref().map(|s| s.item.as_ref()) == operation_name;

                if move_op {
                    operation = Some(op);
                }
            }
            Definition::Fragment(f) => fragments.push(f),
        };
    }

    let op = match operation {
        Some(op) => op,
        None => return Err(GraphQLError::UnknownOperationName),
    };

    let default_variable_values = op.item.variable_definitions.map(|defs| {
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

        let executor = Executor {
            fragments: &fragments
                .iter()
                .map(|f| (f.item.name.item, &f.item))
                .collect(),
            variables: final_vars,
            current_selection_set: Some(&op.item.selection_set[..]),
            schema: &root_node.schema,
            context: context,
            errors: &errors,
            field_path: FieldPath::Root(op.start),
        };

        value = match op.item.operation_type {
            OperationType::Query => executor.resolve_into_value(&root_node.query_info, &root_node),
            OperationType::Mutation => {
                executor.resolve_into_value(&root_node.mutation_info, &root_node.mutation_type)
            }
        };
    }

    let mut errors = errors.into_inner().unwrap();
    errors.sort();

    Ok((value, errors))
}

impl<'r> Registry<'r> {
    /// Construct a new registry
    pub fn new(types: HashMap<Name, MetaType<'r>>) -> Registry<'r> {
        Registry { types: types }
    }

    /// Get the `Type` instance for a given GraphQL type
    ///
    /// If the registry hasn't seen a type with this name before, it will
    /// construct its metadata and store it.
    pub fn get_type<T>(&mut self, info: &T::TypeInfo) -> Type<'r>
    where
        T: GraphQLType,
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
        T: GraphQLType,
    {
        Field {
            name: name.to_owned(),
            description: None,
            arguments: None,
            field_type: self.get_type::<T>(info),
            deprecation_reason: None,
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
            deprecation_reason: None,
        }
    }

    /// Create an argument with the provided name
    pub fn arg<T>(&mut self, name: &str, info: &T::TypeInfo) -> Argument<'r>
    where
        T: GraphQLType + FromInputValue,
    {
        Argument::new(name, self.get_type::<T>(info))
    }

    /// Create an argument with a default value
    ///
    /// When called with type `T`, the actual argument will be given the type
    /// `Option<T>`.
    pub fn arg_with_default<T>(&mut self, name: &str, value: &T, info: &T::TypeInfo) -> Argument<'r>
    where
        T: GraphQLType + ToInputValue + FromInputValue,
    {
        Argument::new(name, self.get_type::<Option<T>>(info)).default_value(value.to())
    }

    fn insert_placeholder(&mut self, name: Name, of_type: Type<'r>) {
        if !self.types.contains_key(&name) {
            self.types.insert(
                name,
                MetaType::Placeholder(PlaceholderMeta { of_type: of_type }),
            );
        }
    }

    /// Create a scalar meta type
    ///
    /// This expects the type to implement `FromInputValue`.
    pub fn build_scalar_type<T>(&mut self, info: &T::TypeInfo) -> ScalarMeta<'r>
    where
        T: FromInputValue + GraphQLType,
    {
        let name = T::name(info).expect("Scalar types must be named. Implement name()");
        ScalarMeta::new::<T>(Cow::Owned(name.to_string()))
    }

    /// Create a list meta type
    pub fn build_list_type<T: GraphQLType>(&mut self, info: &T::TypeInfo) -> ListMeta<'r> {
        let of_type = self.get_type::<T>(info);
        ListMeta::new(of_type)
    }

    /// Create a nullable meta type
    pub fn build_nullable_type<T: GraphQLType>(&mut self, info: &T::TypeInfo) -> NullableMeta<'r> {
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
        fields: &[Field<'r>],
    ) -> ObjectMeta<'r>
    where
        T: GraphQLType,
    {
        let name = T::name(info).expect("Object types must be named. Implement name()");

        let mut v = fields.to_vec();
        v.push(self.field::<String>("__typename", &()));
        ObjectMeta::new(Cow::Owned(name.to_string()), &v)
    }

    /// Create an enum meta type
    pub fn build_enum_type<T>(&mut self, info: &T::TypeInfo, values: &[EnumValue]) -> EnumMeta<'r>
    where
        T: FromInputValue + GraphQLType,
    {
        let name = T::name(info).expect("Enum types must be named. Implement name()");

        EnumMeta::new::<T>(Cow::Owned(name.to_string()), values)
    }

    /// Create an interface meta type builder,
    /// by providing a type info object.
    pub fn build_interface_type<T>(
        &mut self,
        info: &T::TypeInfo,
        fields: &[Field<'r>],
    ) -> InterfaceMeta<'r>
    where
        T: GraphQLType,
    {
        let name = T::name(info).expect("Interface types must be named. Implement name()");

        let mut v = fields.to_vec();
        v.push(self.field::<String>("__typename", &()));
        InterfaceMeta::new(Cow::Owned(name.to_string()), &v)
    }

    /// Create a union meta type builder
    pub fn build_union_type<T>(&mut self, info: &T::TypeInfo, types: &[Type<'r>]) -> UnionMeta<'r>
    where
        T: GraphQLType,
    {
        let name = T::name(info).expect("Union types must be named. Implement name()");

        UnionMeta::new(Cow::Owned(name.to_string()), types)
    }

    /// Create an input object meta type builder
    pub fn build_input_object_type<T>(
        &mut self,
        info: &T::TypeInfo,
        args: &[Argument<'r>],
    ) -> InputObjectMeta<'r>
    where
        T: FromInputValue + GraphQLType,
    {
        let name = T::name(info).expect("Input object types must be named. Implement name()");

        InputObjectMeta::new::<T>(Cow::Owned(name.to_string()), args)
    }
}
