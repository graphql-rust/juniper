//! Code generation for `#[graphql_object]` macro.

use crate::result::GraphQLScope;

/// [`GraphQLScope`] of errors for `#[graphql_object]` macro.
const ERR: GraphQLScope = GraphQLScope::ObjectAttr;
