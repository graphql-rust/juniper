//! Execution of a GraphQL operation that passed parsing and validation.

use std::{
    borrow::Cow,
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
};

use crate::{
    GraphQLError,
    ast::{Definition, Fragment, Operation, OperationType, OwnedDocument},
    parser::Spanning,
    schema::model::{RootNode, TypeType},
    types::{
        async_await::GraphQLTypeAsync, base::GraphQLType, subscriptions::GraphQLSubscriptionType,
    },
    validation::validate_input_values,
    value::{DefaultScalarValue, ScalarValue, Value},
};

use super::{ExecutionError, Executor, FieldPath, ValuesStream, Variables};

/// GraphQL operation that passed parsing and validation.
///
/// [`parse_and_validate()`] returns a [`ValidatedOperation`]. The execution methods validate the
/// variables and execute the operation without parsing and validating the document again. A
/// [`ValidatedOperation`] can execute more than once. Each execution takes its own variables and
/// context.
///
/// [`parse_and_validate()`]: crate::parse_and_validate
pub struct ValidatedOperation<'a, QueryT, MutationT, SubscriptionT, S = DefaultScalarValue>
where
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S>,
    SubscriptionT: GraphQLType<S>,
    S: ScalarValue,
{
    /// Operation to execute.
    operation: Spanning<Operation<'a, S>>,

    /// Fragments of the document by name.
    fragments: HashMap<&'a str, Fragment<'a, S>>,

    /// Schema of the document.
    root_node: &'a RootNode<QueryT, MutationT, SubscriptionT, S>,
}

impl<'a, QueryT, MutationT, SubscriptionT, S>
    ValidatedOperation<'a, QueryT, MutationT, SubscriptionT, S>
where
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S>,
    SubscriptionT: GraphQLType<S>,
    S: ScalarValue,
{
    /// Selects the operation to execute from a `document` that passed validation.
    ///
    /// `operation_name` selects the operation of a document with more than one operation.
    ///
    /// # Errors
    ///
    /// - [`GraphQLError::MultipleOperationsProvided`] if the `document` has more than one operation
    ///   and `operation_name` is [`None`].
    /// - [`GraphQLError::UnknownOperationName`] if the `document` has no operation with the
    ///   `operation_name`.
    pub(crate) fn new(
        document: OwnedDocument<'a, S>,
        operation_name: Option<&str>,
        root_node: &'a RootNode<QueryT, MutationT, SubscriptionT, S>,
    ) -> Result<Self, GraphQLError> {
        let mut operation = None;
        let mut fragments = HashMap::new();
        for definition in document {
            match definition {
                Definition::Operation(candidate) => {
                    if operation_name.is_none() && operation.is_some() {
                        return Err(GraphQLError::MultipleOperationsProvided);
                    }
                    if operation_name.is_none()
                        || candidate.item.name.as_ref().map(|name| name.item) == operation_name
                    {
                        operation = Some(candidate);
                    }
                }
                Definition::Fragment(fragment) => {
                    fragments.insert(fragment.item.name.item, fragment.item);
                }
            }
        }

        Ok(Self {
            operation: operation.ok_or(GraphQLError::UnknownOperationName)?,
            fragments,
            root_node,
        })
    }

    /// Returns the [`OperationType`] of this [`ValidatedOperation`].
    #[must_use]
    pub fn operation_type(&self) -> OperationType {
        self.operation.item.operation_type
    }

    /// Executes this [`ValidatedOperation`] synchronously with the provided `variables` and
    /// `context`.
    ///
    /// # Errors
    ///
    /// - [`GraphQLError::ValidationError`] if the `variables` do not match the variable definitions
    ///   of the operation.
    /// - [`GraphQLError::IsSubscription`] for a subscription.
    /// - [`GraphQLError::NotSupported`] for a mutation if the schema has no mutation root type.
    pub fn execute_sync(
        &self,
        variables: &Variables<S>,
        context: &QueryT::Context,
    ) -> Result<(Value<S>, Vec<ExecutionError<S>>), GraphQLError>
    where
        MutationT: GraphQLType<S, Context = QueryT::Context>,
        SubscriptionT: GraphQLType<S, Context = QueryT::Context>,
    {
        self.validate_variables(variables)?;
        let variables = self.with_default_values(variables);
        let errors = RwLock::new(Vec::new());
        let root_node = self.root_node;

        let value = match self.operation_type() {
            OperationType::Query => self
                .executor(root_node.schema.query_type(), &variables, context, &errors)
                .resolve_into_value(&root_node.query_info, &root_node),
            OperationType::Mutation => self
                .executor(mutation_type(root_node)?, &variables, context, &errors)
                .resolve_into_value(&root_node.mutation_info, &root_node.mutation_type),
            OperationType::Subscription => return Err(GraphQLError::IsSubscription),
        };

        Ok((value, sorted(errors)))
    }

    /// Executes this [`ValidatedOperation`] asynchronously with the provided `variables` and
    /// `context`.
    ///
    /// # Errors
    ///
    /// - [`GraphQLError::ValidationError`] if the `variables` do not match the variable definitions
    ///   of the operation.
    /// - [`GraphQLError::IsSubscription`] for a subscription.
    /// - [`GraphQLError::NotSupported`] for a mutation if the schema has no mutation root type.
    pub async fn execute(
        &self,
        variables: &Variables<S>,
        context: &QueryT::Context,
    ) -> Result<(Value<S>, Vec<ExecutionError<S>>), GraphQLError>
    where
        QueryT: GraphQLTypeAsync<S>,
        QueryT::TypeInfo: Sync,
        QueryT::Context: Sync,
        MutationT: GraphQLTypeAsync<S, Context = QueryT::Context>,
        MutationT::TypeInfo: Sync,
        SubscriptionT: GraphQLType<S, Context = QueryT::Context> + Sync,
        SubscriptionT::TypeInfo: Sync,
        S: Send + Sync,
    {
        self.validate_variables(variables)?;
        let variables = self.with_default_values(variables);
        let errors = RwLock::new(Vec::new());
        let root_node = self.root_node;

        let value = match self.operation_type() {
            OperationType::Query => {
                self.executor(root_node.schema.query_type(), &variables, context, &errors)
                    .resolve_into_value_async(&root_node.query_info, &root_node)
                    .await
            }
            OperationType::Mutation => {
                self.executor(mutation_type(root_node)?, &variables, context, &errors)
                    .resolve_into_value_async(&root_node.mutation_info, &root_node.mutation_type)
                    .await
            }
            OperationType::Subscription => return Err(GraphQLError::IsSubscription),
        };

        Ok((value, sorted(errors)))
    }

    /// Resolves this subscription [`ValidatedOperation`] into a [`ValuesStream`] with the provided
    /// `variables` and `context`.
    ///
    /// # Errors
    ///
    /// - [`GraphQLError::ValidationError`] if the `variables` do not match the variable definitions
    ///   of the operation.
    /// - [`GraphQLError::NotSubscription`] for a query or a mutation.
    /// - [`GraphQLError::NotSupported`] if the schema has no subscription root type.
    pub async fn resolve_into_stream(
        &self,
        variables: &Variables<S>,
        context: &'a QueryT::Context,
    ) -> Result<(Value<ValuesStream<'a, S>>, Vec<ExecutionError<S>>), GraphQLError>
    where
        QueryT: GraphQLTypeAsync<S>,
        QueryT::TypeInfo: Sync,
        QueryT::Context: Sync,
        MutationT: GraphQLTypeAsync<S, Context = QueryT::Context>,
        MutationT::TypeInfo: Sync,
        SubscriptionT: GraphQLSubscriptionType<S, Context = QueryT::Context>,
        SubscriptionT::TypeInfo: Sync,
        S: Send + Sync,
    {
        self.validate_variables(variables)?;
        if self.operation_type() != OperationType::Subscription {
            return Err(GraphQLError::NotSubscription);
        }
        let root_node = self.root_node;
        let root_type = root_node
            .schema
            .subscription_type()
            .ok_or(GraphQLError::NotSupported(OperationType::Subscription))?;
        let variables = self.with_default_values(variables);
        let errors = RwLock::new(Vec::new());

        let value = self
            .executor(root_type, &variables, context, &errors)
            .resolve_into_stream(&root_node.subscription_info, &root_node.subscription_type)
            .await;

        Ok((value, sorted(errors)))
    }

    /// Validates the `variables` against the variable definitions of the operation.
    fn validate_variables(&self, variables: &Variables<S>) -> Result<(), GraphQLError> {
        let errors = validate_input_values(variables, &self.operation, &self.root_node.schema);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.into())
        }
    }

    /// Adds the default values of the operation for the variables missing from the `variables`.
    ///
    /// Copies the `variables` only if a default value applies.
    fn with_default_values<'v>(&self, variables: &'v Variables<S>) -> Cow<'v, Variables<S>> {
        let mut variables = Cow::Borrowed(variables);
        let Some(definitions) = &self.operation.item.variables_definition else {
            return variables;
        };
        for (name, definition) in &definitions.item.items {
            let Some(default) = &definition.default_value else {
                continue;
            };
            if !variables.contains_key(name.item) {
                variables
                    .to_mut()
                    .insert(name.item.into(), default.item.clone());
            }
        }
        variables
    }

    /// Creates an [`Executor`] for the root selection set of the operation.
    fn executor<'r, 'x>(
        &'r self,
        root_type: TypeType<'x, S>,
        variables: &'r Variables<S>,
        context: &'x QueryT::Context,
        errors: &'r RwLock<Vec<ExecutionError<S>>>,
    ) -> Executor<'r, 'x, QueryT::Context, S>
    where
        'a: 'x,
    {
        Executor {
            fragments: &self.fragments,
            variables,
            current_selection_set: Some(&self.operation.item.selection_set),
            parent_selection_set: None,
            current_type: root_type,
            schema: &self.root_node.schema,
            context,
            errors,
            field_path: Arc::new(FieldPath::Root(self.operation.span.start)),
            serial: self.operation_type() == OperationType::Mutation,
        }
    }
}

impl<QueryT, MutationT, SubscriptionT, S> fmt::Debug
    for ValidatedOperation<'_, QueryT, MutationT, SubscriptionT, S>
where
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S>,
    SubscriptionT: GraphQLType<S>,
    S: ScalarValue,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ValidatedOperation")
            .field("operation", &self.operation)
            .field("fragments", &self.fragments)
            .finish_non_exhaustive()
    }
}

/// Returns the mutation root type of the `root_node`.
///
/// # Errors
///
/// [`GraphQLError::NotSupported`] if the schema has no mutation root type. An [`EmptyMutation`]
/// root has no type in the schema.
///
/// [`EmptyMutation`]: crate::EmptyMutation
fn mutation_type<QueryT, MutationT, SubscriptionT, S>(
    root_node: &RootNode<QueryT, MutationT, SubscriptionT, S>,
) -> Result<TypeType<'_, S>, GraphQLError>
where
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S>,
    SubscriptionT: GraphQLType<S>,
    S: ScalarValue,
{
    root_node
        .schema
        .mutation_type()
        .ok_or(GraphQLError::NotSupported(OperationType::Mutation))
}

/// Returns the `errors` of an execution in the order of their positions in the document.
fn sorted<S: ScalarValue>(errors: RwLock<Vec<ExecutionError<S>>>) -> Vec<ExecutionError<S>> {
    let mut errors = errors.into_inner().unwrap();
    errors.sort();
    errors
}
