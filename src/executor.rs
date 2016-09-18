use std::collections::HashMap;
use std::marker::PhantomData;

use ast::{InputValue, ToInputValue, Document, Selection, Fragment, Definition, Type, FromInputValue, OperationType};
use value::Value;
use parser::SourcePosition;

use schema::meta::{MetaType, ScalarMeta, ListMeta, NullableMeta,
                   ObjectMeta, EnumMeta, InterfaceMeta, UnionMeta,
                   InputObjectMeta, PlaceholderMeta, Field, Argument,
                   EnumValue};
use schema::model::{RootNode, SchemaType};

use types::base::GraphQLType;

/// A type registry used to build schemas
///
/// The registry gathers metadata for all types in a schema. It provides
/// convenience methods to convert types implementing the `GraphQLType` trait
/// into `Type` instances and automatically registers them.
pub struct Registry<CtxT> {
    /// Currently registered types
    pub types: HashMap<String, MetaType>,
    phantom: PhantomData<CtxT>,
}

#[derive(Clone)]
pub enum FieldPath<'a> {
    Root(SourcePosition),
    Field(String, SourcePosition, &'a FieldPath<'a>),
}

/// Query execution engine
///
/// The executor helps drive the query execution in a schema. It keeps track
/// of the current field stack, context, variables, and errors.
pub struct Executor<'a, CtxT> where CtxT: 'a {
    fragments: &'a HashMap<String, Fragment>,
    variables: &'a HashMap<String, InputValue>,
    current_selection_set: Option<Vec<Selection>>,
    schema: &'a SchemaType,
    context: &'a CtxT,
    errors: &'a mut Vec<ExecutionError>,
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

impl<'a, CtxT> Executor<'a, CtxT> {
    /// Resolve a single arbitrary value into an `ExecutionResult`
    pub fn resolve<T: GraphQLType<CtxT>>(&mut self, value: &T) -> ExecutionResult {
        Ok(value.resolve(
            match self.current_selection_set {
                Some(ref sel) => Some(sel.clone()),
                None => None,
            },
            self))
    }

    /// Resolve a single arbitrary value into a return value
    ///
    /// If the field fails to resolve, `null` will be returned.
    pub fn resolve_into_value<T: GraphQLType<CtxT>>(&mut self, value: &T) -> Value {
        match self.resolve(value) {
            Ok(v) => v,
            Err(e) => {
                let position = self.field_path.location().clone();
                self.push_error(e, position);
                Value::null()
            },
        }
    }

    /// Derive a new executor by replacing the context
    ///
    /// This can be used to connect different types, e.g. from different Rust
    /// libraries, that require different context types.
    pub fn replaced_context<'b, NewCtxT>(&'b mut self, ctx: &'b NewCtxT) -> Executor<'b, NewCtxT> {
        Executor {
            fragments: self.fragments,
            variables: self.variables,
            current_selection_set: self.current_selection_set.clone(),
            schema: self.schema,
            context: ctx,
            errors: self.errors,
            field_path: self.field_path.clone(),
        }
    }

    #[doc(hidden)]
    pub fn sub_executor(
        &mut self,
        field_name: Option<String>,
        location: SourcePosition,
        selection_set: Option<Vec<Selection>>,
    )
        -> Executor<CtxT>
    {
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
    pub fn variables(&self) -> &'a HashMap<String, InputValue> {
        self.variables
    }

    #[doc(hidden)]
    pub fn fragment_by_name(&self, name: &str) -> Option<&'a Fragment> {
        self.fragments.get(name)
    }

    /// Add an error to the execution engine
    pub fn push_error(&mut self, error: String, location: SourcePosition) {
        let mut path = Vec::new();
        self.field_path.construct_path(&mut path);

        self.errors.push(ExecutionError {
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
            FieldPath::Field(ref name, _, ref parent) => {
                parent.construct_path(acc);
                acc.push(name.clone());
            }
        }
    }

    fn location(&self) -> &SourcePosition {
        match *self {
            FieldPath::Root(ref pos) |
            FieldPath::Field(_, ref pos, _) => pos
        }
    }
}

impl ExecutionError {
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

pub fn execute_validated_query<QueryT, MutationT, CtxT>(
    document: Document,
    operation_name: Option<&str>,
    root_node: &RootNode<CtxT, QueryT, MutationT>,
    variables: &HashMap<String, InputValue>,
    context: &CtxT
)
    -> (Value, Vec<ExecutionError>)
    where QueryT: GraphQLType<CtxT>,
          MutationT: GraphQLType<CtxT>,
{
    let mut fragments = vec![];
    let mut operation = None;

    for def in document {
        match def {
            Definition::Operation(op) => {
                if operation_name.is_none() && operation.is_some() {
                    panic!("Must provide operation name if query contains multiple operations");
                }

                let move_op = operation_name.is_none()
                    || op.item.name.as_ref().map(|s| s.item.as_ref()) == operation_name;

                if move_op {
                    operation = Some(op);
                }
            }
            Definition::Fragment(f) => fragments.push(f),
        };
    }

    let op = operation.expect("Could not find operation to execute");
    let mut errors = Vec::new();
    let value;

    {
        let mut executor = Executor {
            fragments: &fragments.into_iter().map(|f| (f.item.name.item.clone(), f.item)).collect(),
            variables: variables,
            current_selection_set: Some(op.item.selection_set),
            schema: &root_node.schema,
            context: context,
            errors: &mut errors,
            field_path: FieldPath::Root(op.start),
        };

        value = match op.item.operation_type {
            OperationType::Query => executor.resolve_into_value(&root_node),
            OperationType::Mutation => executor.resolve_into_value(&root_node.mutation_type),
        };
    }

    errors.sort();

    (value, errors)
}

impl<CtxT> Registry<CtxT> {
    /// Construct a new registry
    pub fn new(types: HashMap<String, MetaType>) -> Registry<CtxT> {
        Registry {
            types: types,
            phantom: PhantomData,
        }
    }

    /// Get the `Type` instance for a given GraphQL type
    ///
    /// If the registry hasn't seen a type with this name before, it will
    /// construct its metadata and store it.
    pub fn get_type<T>(&mut self) -> Type where T: GraphQLType<CtxT> {
        if let Some(name) = T::name() {
            if !self.types.contains_key(name) {
                self.insert_placeholder(name, Type::NonNullNamed(name.to_owned()));
                let meta = T::meta(self);
                self.types.insert(name.to_owned(), meta);
            }
            self.types[name].as_type()
        }
        else {
            T::meta(self).as_type()
        }
    }

    /// Create a field with the provided name
    pub fn field<T>(&mut self, name: &str) -> Field where T: GraphQLType<CtxT> {
        Field {
            name: name.to_owned(),
            description: None,
            arguments: None,
            field_type: self.get_type::<T>(),
            deprecation_reason: None,
        }
    }

    #[doc(hidden)]
    pub fn field_inside_result<T>(&mut self, name: &str, _: FieldResult<T>) -> Field where T: GraphQLType<CtxT> {
        Field {
            name: name.to_owned(),
            description: None,
            arguments: None,
            field_type: self.get_type::<T>(),
            deprecation_reason: None,
        }
    }

    /// Create an argument with the provided name
    pub fn arg<T>(&mut self, name: &str) -> Argument where T: GraphQLType<CtxT> + FromInputValue {
        Argument::new(name, self.get_type::<T>())
    }

    /// Create an argument with a default value
    ///
    /// When called with type `T`, the actual argument will be given the type
    /// `Option<T>`.
    pub fn arg_with_default<T>(
        &mut self,
        name: &str,
        value: &T,
    )
        -> Argument
        where T: GraphQLType<CtxT> + ToInputValue + FromInputValue
    {
        Argument::new(name, self.get_type::<Option<T>>())
            .default_value(value.to())
    }

    fn insert_placeholder(&mut self, name: &str, of_type: Type) {
        if !self.types.contains_key(name) {
            self.types.insert(
                name.to_owned(),
                MetaType::Placeholder(PlaceholderMeta { of_type: of_type }));
        }
    }

    /// Create a scalar meta type
    ///
    /// This expects the type to implement `FromInputValue`.
    pub fn build_scalar_type<T>(&mut self)
        -> ScalarMeta
        where T: FromInputValue + GraphQLType<CtxT>
    {
        let name = T::name().expect("Scalar types must be named. Implement name()");
        ScalarMeta::new::<T>(name)
    }

    /// Create a list meta type
    pub fn build_list_type<T: GraphQLType<CtxT>>(&mut self) -> ListMeta {
        let of_type = self.get_type::<T>();
        ListMeta::new(of_type)
    }

    /// Create a nullable meta type
    pub fn build_nullable_type<T: GraphQLType<CtxT>>(&mut self) -> NullableMeta {
        let of_type = self.get_type::<T>();
        NullableMeta::new(of_type)
    }

    /// Create an object meta type builder
    ///
    /// To prevent infinite recursion by enforcing ordering, this returns a
    /// function that needs to be called with the list of fields on the object.
    pub fn build_object_type<T>(&mut self)
        -> Box<Fn(&[Field]) -> ObjectMeta>
        where T: GraphQLType<CtxT>
    {
        let name = T::name().expect("Object types must be named. Implement name()");
        let typename_field = self.field::<String>("__typename");

        Box::new(move |fs: &[Field]| {
            let mut v = fs.to_vec();
            v.push(typename_field.clone());
            ObjectMeta::new(name, &v)
        })
    }

    /// Create an enum meta type
    pub fn build_enum_type<T>(&mut self)
        -> Box<Fn(&[EnumValue]) -> EnumMeta>
        where T: FromInputValue + GraphQLType<CtxT>
    {
        let name = T::name().expect("Enum types must be named. Implement name()");

        Box::new(move |values: &[EnumValue]| EnumMeta::new::<T>(name, values))
    }

    /// Create an interface meta type builder
    pub fn build_interface_type<T>(&mut self)
        -> Box<Fn(&[Field]) -> InterfaceMeta>
        where T: GraphQLType<CtxT>
    {
        let name = T::name().expect("Interface types must be named. Implement name()");
        let typename_field = self.field::<String>("__typename");

        Box::new(move |fs: &[Field]| {
            let mut v = fs.to_vec();
            v.push(typename_field.clone());
            InterfaceMeta::new(name, &v)
        })
    }

    /// Create a union meta type builder
    pub fn build_union_type<T>(&mut self)
        -> Box<Fn(&[Type]) -> UnionMeta>
        where T: GraphQLType<CtxT>
    {
        let name = T::name().expect("Union types must be named. Implement name()");

        Box::new(move |ts: &[Type]| UnionMeta::new(name, ts))
    }

    /// Create an input object meta type builder
    pub fn build_input_object_type<T>(&mut self)
        -> Box<Fn(&[Argument]) -> InputObjectMeta>
        where T: FromInputValue + GraphQLType<CtxT>
    {
        let name = T::name().expect("Input object types must be named. Implement name()");

        Box::new(move |args: &[Argument]| InputObjectMeta::new::<T>(name, args))
    }
}
