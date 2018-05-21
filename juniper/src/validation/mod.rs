//! Query validation related methods and data structures

mod context;
mod input_value;
mod multi_visitor;
mod rules;
mod traits;
mod visitor;

#[cfg(test)]
mod test_harness;

pub use self::context::{RuleError, ValidatorContext};
pub use self::input_value::validate_input_values;
pub use self::multi_visitor::{MultiVisitor, MultiVisitorNil};
pub use self::rules::visit_all_rules;
pub use self::traits::Visitor;
pub use self::visitor::visit;

#[cfg(test)]
pub use self::test_harness::{expect_fails_rule, expect_fails_rule_with_schema, expect_passes_rule,
                             expect_passes_rule_with_schema};
