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
    executor::{execute_validated_query, get_operation},
    introspection::{INTROSPECTION_QUERY, INTROSPECTION_QUERY_WITHOUT_DESCRIPTIONS},
    parser::parse_document_source,
    validation::{
        MultiVisitorNil, ValidatorContext, rules, validate_input_values, visit as visit_rule,
        visit_all_rules,
    },
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
        Registry, ValuesStream, Variables,
    },
    introspection::IntrospectionFormat,
    macros::helper::subscription::{ExtractTypeFromStream, IntoFieldResult},
    parser::{ParseError, ScalarToken, Span, Spanning, StringLiteral},
    schema::{
        meta,
        model::{RootNode, SchemaType, DirectiveType, DirectiveLocation},
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
            | Self::NotSubscription => None,
        }
    }
}

/// Execute a query synchronously in a provided schema
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

    let operation = get_operation(&document, operation_name)?;

    {
        let errors = validate_input_values(variables, operation, &root_node.schema);

        if !errors.is_empty() {
            return Err(errors.into());
        }
    }

    execute_validated_query(&document, operation, root_node, variables, context)
}

/// Execute a query in a provided schema
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

    let operation = get_operation(&document, operation_name)?;

    {
        let errors = validate_input_values(variables, operation, &root_node.schema);

        if !errors.is_empty() {
            return Err(errors.into());
        }
    }

    executor::execute_validated_query_async(&document, operation, root_node, variables, context)
        .await
}

/// Resolve subscription into `ValuesStream`
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
    let document: ast::OwnedDocument<'a, S> =
        parse_document_source(document_source, &root_node.schema)?;

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

    let operation = get_operation(&document, operation_name)?;

    {
        let errors = validate_input_values(variables, operation, &root_node.schema);

        if !errors.is_empty() {
            return Err(errors.into());
        }
    }

    executor::resolve_validated_subscription(&document, operation, root_node, variables, context)
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
