#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(any(doc, test), doc = include_str!("../README.md"))]
#![cfg_attr(not(any(doc, test)), doc = env!("CARGO_PKG_NAME"))]
// Due to `schema_introspection` test.
#![cfg_attr(test, recursion_limit = "256")]

// Required for using `juniper_codegen` macros inside this crate to resolve
// absolute `::juniper` path correctly, without errors.
extern crate core;
extern crate self as juniper;

#[cfg(test)]
mod for_benches_only {
    use bencher as _;
}
#[cfg(test)]
mod for_feature_gated_tests_only {
    #[cfg(not(feature = "chrono"))]
    use chrono as _;
    #[cfg(not(feature = "jiff"))]
    use jiff as _;
    #[cfg(not(feature = "anyhow"))]
    use serial_test as _;
}

// These are required by the code generated via the `juniper_codegen` macros.
#[doc(hidden)]
pub use {async_trait::async_trait, futures, serde, static_assertions as sa};

#[doc(inline)]
pub use futures::future::{BoxFuture, LocalBoxFuture};

pub use arcstr::{self, ArcStr};

// Depend on juniper_codegen and re-export everything in it.
// This allows users to just depend on juniper and get the derive
// functionality automatically.
pub use juniper_codegen::{
    GraphQLEnum, GraphQLInputObject, GraphQLInterface, GraphQLObject, GraphQLScalar, GraphQLUnion,
    graphql_interface, graphql_object, graphql_scalar, graphql_subscription, graphql_union,
};

#[doc(hidden)]
#[macro_use]
pub mod macros;
mod ast;
pub mod executor;
#[cfg(test)]
pub(crate) mod graphql;
mod introspection;
pub mod parser;
pub(crate) mod schema;
mod types;
mod util;
pub mod validation;
mod value;
// This needs to be public until docs have support for private modules:
// https://github.com/rust-lang/cargo/issues/1520
pub mod http;
pub mod integrations;

#[cfg(all(test, not(feature = "expose-test-schema")))]
mod tests;
#[cfg(feature = "expose-test-schema")]
pub mod tests;

#[cfg(test)]
mod executor_tests;

use derive_more::with_trait::{Display, From};
use itertools::Itertools as _;

// Needs to be public because macros use it.
pub use crate::util::to_camel_case;

use crate::{
    introspection::{INTROSPECTION_QUERY, INTROSPECTION_QUERY_WITHOUT_DESCRIPTIONS},
    parser::parse_document_source,
    validation::{MultiVisitorNil, ValidatorContext, rules, visit as visit_rule, visit_all_rules},
};

pub use crate::{
    ast::{
        Definition, Document, FromInputValue, InputValue, IntoInputValue, Operation, OperationType,
        Selection, ToInputValue, Type,
    },
    executor::{
        Applies, Context, ExecutionError, ExecutionResult, Executor, FieldError, FieldResult,
        FromContext, IntoFieldError, IntoResolvable, LookAheadArgument, LookAheadChildren,
        LookAheadList, LookAheadObject, LookAheadSelection, LookAheadValue, OwnedExecutor,
        Registry, ValidatedOperation, ValuesStream, Variables,
    },
    introspection::IntrospectionFormat,
    macros::helper::subscription::{ExtractTypeFromStream, IntoFieldResult},
    parser::{ParseError, ScalarToken, Span, Spanning, StringLiteral},
    schema::{
        meta,
        model::{RootNode, SchemaType},
    },
    types::{
        async_await::{GraphQLTypeAsync, GraphQLValueAsync},
        base::{Arguments, GraphQLType, GraphQLValue, TypeKind},
        marker::{self, GraphQLInterface, GraphQLObject, GraphQLUnion},
        nullable::Nullable,
        scalars::{EmptyMutation, EmptySubscription, ID},
        subscriptions::{
            ExecutionOutput, GraphQLSubscriptionType, GraphQLSubscriptionValue,
            SubscriptionConnection, SubscriptionCoordinator,
        },
    },
    validation::RuleError,
    value::{
        AnyExt, DefaultScalarValue, FromScalarValue, IntoValue, Object, ParseScalarResult,
        ParseScalarValue, Scalar, ScalarValue, ToScalarValue, TryToPrimitive, Value,
        WrongInputScalarTypeError,
    },
};

/// An error that prevented query execution
#[expect(missing_docs, reason = "self-explanatory")]
#[derive(Clone, Debug, Display, Eq, From, PartialEq)]
pub enum GraphQLError {
    ParseError(Spanning<ParseError>),
    #[display("{}", _0.iter().format("\n"))]
    ValidationError(Vec<RuleError>),
    #[display("No operation provided")]
    NoOperationProvided,
    #[display("Multiple operations provided")]
    MultipleOperationsProvided,
    #[display("Unknown operation name")]
    UnknownOperationName,
    #[display("Operation is a subscription")]
    IsSubscription,
    #[display("Operation is not a subscription")]
    NotSubscription,
    /// The requested operation type has no corresponding root type defined in
    /// the schema — e.g. a `mutation` against [`EmptyMutation`], or a
    /// `subscription` against [`EmptySubscription`].
    ///
    /// [`EmptyMutation`]: crate::EmptyMutation
    /// [`EmptySubscription`]: crate::EmptySubscription
    #[display("Schema is not configured for {} operations", match _0 {
        OperationType::Query => "queries",
        OperationType::Mutation => "mutations",
        OperationType::Subscription => "subscriptions",
    })]
    NotSupported(OperationType),
}

impl From<RuleError> for GraphQLError {
    fn from(value: RuleError) -> Self {
        vec![value].into()
    }
}

impl std::error::Error for GraphQLError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ParseError(e) => Some(e),
            Self::ValidationError(errs) => Some(errs.first()?),
            Self::NoOperationProvided
            | Self::MultipleOperationsProvided
            | Self::UnknownOperationName
            | Self::IsSubscription
            | Self::NotSubscription
            | Self::NotSupported(_) => None,
        }
    }
}

/// Parses and validates a GraphQL document in the provided schema without executing it.
///
/// `operation_name` selects the operation of a document with more than one operation. The returned
/// [`ValidatedOperation`] reports the [`OperationType`] of the selected operation. The execution
/// methods of the [`ValidatedOperation`] execute the operation without parsing and validating the
/// document again. A server can thus route subscriptions to
/// [`ValidatedOperation::resolve_into_stream()`] and other operations to
/// [`ValidatedOperation::execute()`].
///
/// # Errors
///
/// - [`GraphQLError::ParseError`] if the document fails to parse or exceeds the document limits of
///   the `root_node`.
/// - [`GraphQLError::ValidationError`] if the document fails validation.
/// - [`GraphQLError::MultipleOperationsProvided`] if the document has more than one operation and
///   `operation_name` is [`None`].
/// - [`GraphQLError::UnknownOperationName`] if the document has no operation with the
///   `operation_name`.
///
/// # Example
///
/// ```rust
/// # use juniper::{
/// #     EmptyMutation, EmptySubscription, GraphQLError, OperationType, RootNode, graphql_object,
/// #     graphql_value, graphql_vars,
/// # };
/// #
/// struct Query;
///
/// #[graphql_object]
/// impl Query {
///     fn ping() -> bool {
///         true
///     }
/// }
///
/// type Schema = RootNode<Query, EmptyMutation, EmptySubscription>;
///
/// let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new());
///
/// let operation = juniper::parse_and_validate("{ ping }", None, &schema)?;
/// assert_eq!(operation.operation_type(), OperationType::Query);
///
/// let (data, errors) = operation.execute_sync(&graphql_vars! {}, &())?;
/// assert_eq!(data, graphql_value!({"ping": true}));
/// assert!(errors.is_empty());
/// # Ok::<_, GraphQLError>(())
/// ```
pub fn parse_and_validate<'a, S, QueryT, MutationT, SubscriptionT>(
    document_source: &'a str,
    operation_name: Option<&str>,
    root_node: &'a RootNode<QueryT, MutationT, SubscriptionT, S>,
) -> Result<ValidatedOperation<'a, QueryT, MutationT, SubscriptionT, S>, GraphQLError>
where
    S: ScalarValue,
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S>,
    SubscriptionT: GraphQLType<S>,
{
    let document = parse_document_source(document_source, &root_node.schema)?;

    {
        let mut ctx = ValidatorContext::new(&root_node.schema, &document);
        visit_all_rules(&mut ctx, &document);
        if root_node.introspection_disabled {
            visit_rule(
                &mut MultiVisitorNil.with(rules::disable_introspection::factory()),
                &mut ctx,
                &document,
            );
        }

        let errors = ctx.into_errors();
        if !errors.is_empty() {
            return Err(errors.into());
        }
    }

    ValidatedOperation::new(document, operation_name, root_node)
}

/// Executes a query synchronously in the provided schema.
///
/// Parses and validates the document with [`parse_and_validate()`] and executes the operation with
/// [`ValidatedOperation::execute_sync()`].
pub fn execute_sync<'a, S, QueryT, MutationT, SubscriptionT>(
    document_source: &'a str,
    operation_name: Option<&str>,
    root_node: &'a RootNode<QueryT, MutationT, SubscriptionT, S>,
    variables: &Variables<S>,
    context: &QueryT::Context,
) -> Result<(Value<S>, Vec<ExecutionError<S>>), GraphQLError>
where
    S: ScalarValue,
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S, Context = QueryT::Context>,
    SubscriptionT: GraphQLType<S, Context = QueryT::Context>,
{
    parse_and_validate(document_source, operation_name, root_node)?.execute_sync(variables, context)
}

/// Executes a query asynchronously in the provided schema.
///
/// Parses and validates the document with [`parse_and_validate()`] and executes the operation with
/// [`ValidatedOperation::execute()`].
pub async fn execute<'a, S, QueryT, MutationT, SubscriptionT>(
    document_source: &'a str,
    operation_name: Option<&str>,
    root_node: &'a RootNode<QueryT, MutationT, SubscriptionT, S>,
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
    S: ScalarValue + Send + Sync,
{
    parse_and_validate(document_source, operation_name, root_node)?
        .execute(variables, context)
        .await
}

/// Resolves a subscription into a [`ValuesStream`] in the provided schema.
///
/// Parses and validates the document with [`parse_and_validate()`] and resolves the operation with
/// [`ValidatedOperation::resolve_into_stream()`].
pub async fn resolve_into_stream<'a, S, QueryT, MutationT, SubscriptionT>(
    document_source: &'a str,
    operation_name: Option<&str>,
    root_node: &'a RootNode<QueryT, MutationT, SubscriptionT, S>,
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
    S: ScalarValue + Send + Sync,
{
    parse_and_validate(document_source, operation_name, root_node)?
        .resolve_into_stream(variables, context)
        .await
}

/// Executes the [canonical introspection query][0] in the provided schema.
///
/// [0]: https://github.com/graphql/graphql-js/blob/v16.11.0/src/utilities/getIntrospectionQuery.ts#L75
pub fn introspect<S, QueryT, MutationT, SubscriptionT>(
    root_node: &RootNode<QueryT, MutationT, SubscriptionT, S>,
    context: &QueryT::Context,
    format: IntrospectionFormat,
) -> Result<(Value<S>, Vec<ExecutionError<S>>), GraphQLError>
where
    S: ScalarValue,
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S, Context = QueryT::Context>,
    SubscriptionT: GraphQLType<S, Context = QueryT::Context>,
{
    execute_sync(
        match format {
            IntrospectionFormat::All => INTROSPECTION_QUERY,
            IntrospectionFormat::WithoutDescriptions => INTROSPECTION_QUERY_WITHOUT_DESCRIPTIONS,
        },
        None,
        root_node,
        &Variables::new(),
        context,
    )
}
