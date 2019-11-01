//! This are just inimplemented `test_harness` functions which
//! are used elsewhere (needed temporarily
//! while `test_harness` does not compile)

use crate::{
    types::base::GraphQLType,
    validation::{RuleError, Visitor},
    value::{ScalarRefValue, ScalarValue},
};

pub fn expect_fails_rule<'a, V, F, S>(factory: F, q: &'a str, expected_errors: &[RuleError])
where
    S: ScalarValue + 'a,
    for<'b> &'b S: ScalarRefValue<'b>,
    V: Visitor<'a, S> + 'a,
    F: Fn() -> V,
{
    unimplemented!()
}

pub fn expect_fails_rule_with_schema<'a, Q, M, V, F, S>(
    r: Q,
    m: M,
    factory: F,
    q: &'a str,
    expected_errors: &[RuleError],
) where
    S: ScalarValue + 'a,
    for<'b> &'b S: ScalarRefValue<'b>,
    Q: GraphQLType<S, TypeInfo = ()>,
    M: GraphQLType<S, TypeInfo = ()>,
    V: Visitor<'a, S> + 'a,
    F: Fn() -> V,
{
    unimplemented!()
}

pub fn expect_passes_rule<'a, V, F, S>(factory: F, q: &'a str)
where
    S: ScalarValue + 'a,
    for<'b> &'b S: ScalarRefValue<'b>,
    V: Visitor<'a, S> + 'a,
    F: Fn() -> V,
{
    unimplemented!();
}

pub fn expect_passes_rule_with_schema<'a, Q, M, V, F, S>(r: Q, m: M, factory: F, q: &'a str)
where
    S: ScalarValue + 'a,
    for<'b> &'b S: ScalarRefValue<'b>,
    Q: GraphQLType<S, TypeInfo = ()>,
    M: GraphQLType<S, TypeInfo = ()>,
    V: Visitor<'a, S> + 'a,
    F: Fn() -> V,
{
    unimplemented!();
}
