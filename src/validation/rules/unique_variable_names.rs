use std::collections::hash_map::{HashMap, Entry};

use ast::{Operation, VariableDefinition};
use parser::{SourcePosition, Spanning};
use validation::{ValidatorContext, Visitor};

pub struct UniqueVariableNames<'a> {
    names: HashMap<&'a str, SourcePosition>,
}

pub fn factory<'a>() -> UniqueVariableNames<'a> {
    UniqueVariableNames {
        names: HashMap::new(),
    }
}

impl<'a> Visitor<'a> for UniqueVariableNames<'a> {
    fn enter_operation_definition(&mut self, _: &mut ValidatorContext<'a>, _: &'a Spanning<Operation>) {
        self.names = HashMap::new();
    }

    fn enter_variable_definition(&mut self, ctx: &mut ValidatorContext<'a>, &(ref var_name, _): &'a (Spanning<String>, VariableDefinition)) {
        match self.names.entry(&var_name.item) {
            Entry::Occupied(e) => {
                ctx.report_error(
                    &error_message(&var_name.item),
                    &[e.get().clone(), var_name.start.clone()]);
            }
            Entry::Vacant(e) => {
                e.insert(var_name.start.clone());
            }
        }
    }
}

fn error_message(var_name: &str) -> String {
    format!("There can only be one variable named {}", var_name)
}

#[cfg(test)]
mod tests {
    use super::{error_message, factory};

    use parser::SourcePosition;
    use validation::{RuleError, expect_passes_rule, expect_fails_rule};

    #[test]
    fn unique_variable_names() {
        expect_passes_rule(factory, r#"
          query A($x: Int, $y: String) { __typename }
          query B($x: String, $y: Int) { __typename }
        "#);
    }

    #[test]
    fn duplicate_variable_names() {
        expect_fails_rule(factory, r#"
          query A($x: Int, $x: Int, $x: String) { __typename }
          query B($x: String, $x: Int) { __typename }
          query C($x: Int, $x: Int) { __typename }
        "#,
            &[
                RuleError::new(&error_message("x"), &[
                    SourcePosition::new(19, 1, 18),
                    SourcePosition::new(28, 1, 27),
                ]),
                RuleError::new(&error_message("x"), &[
                    SourcePosition::new(19, 1, 18),
                    SourcePosition::new(37, 1, 36),
                ]),
                RuleError::new(&error_message("x"), &[
                    SourcePosition::new(82, 2, 18),
                    SourcePosition::new(94, 2, 30),
                ]),
                RuleError::new(&error_message("x"), &[
                    SourcePosition::new(136, 3, 18),
                    SourcePosition::new(145, 3, 27),
                ]),
            ]);
    }
}
