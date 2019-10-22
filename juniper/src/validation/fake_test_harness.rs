use juniper_codegen::GraphQLInputObjectInternal as GraphQLInputObject;

use crate::{
    ast::{FromInputValue, InputValue},
    executor::Registry,
    parser::parse_document_source,
    schema::{
        meta::{EnumValue, MetaType},
        model::{DirectiveLocation, DirectiveType, RootNode},
    },
    types::{base::GraphQLType, scalars::ID},
    validation::{visit, MultiVisitorNil, RuleError, ValidatorContext, Visitor},
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