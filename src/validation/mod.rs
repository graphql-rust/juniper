//! Query validation related methods and data structures

mod visitor;
mod traits;
mod context;
mod multi_visitor;
mod rules;

#[cfg(test)]
mod test_harness;

pub use self::traits::Visitor;
pub use self::visitor::visit;
pub use self::context::{RuleError, ValidatorContext};
pub use self::rules::visit_all_rules;
pub use self::multi_visitor::MultiVisitor;

#[cfg(test)]
pub use self::test_harness::{
    expect_passes_rule, expect_fails_rule,
    expect_passes_rule_with_schema, expect_fails_rule_with_schema};
