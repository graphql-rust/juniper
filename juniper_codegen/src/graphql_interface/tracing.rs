pub use crate::{
    tracing::{
        async_tokens, span_tokens, sync_tokens, Attr, FieldBehavior, Rule, TracedArgument,
        TracedField, TracedType,
    }
};

use super::{Definition as InterfaceDefinition, field};

impl TracedType for InterfaceDefinition {
    fn tracing_rule(&self) -> Rule {
        self.tracing_rule.unwrap_or(Rule::All)
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn scalar(&self) -> Option<syn::Type> {
        Some(self.scalar.ty())
    }
}

impl TracedField for field::Definition {
    type Arg = field::arg::OnField;

    fn instrument(&self) -> Option<&Attr> {
        self.instrument.as_ref()
    }

    fn tracing_behavior(&self) -> FieldBehavior {
        self.tracing.unwrap_or(FieldBehavior::Default)
    }

    fn is_async(&self) -> bool {
        self.is_async
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn args(&self) -> Vec<&Self::Arg> {
        self.arguments
            .as_ref()
            .map_or_else(
                || vec![],
                |args| args.iter()
                    .filter_map(|arg| arg.as_regular())
                    .collect())
    }
}

impl TracedArgument for field::arg::OnField {
    fn ty(&self) -> &syn::Type {
        &self.ty
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn raw_name(&self) -> &syn::Ident {
        &self.raw_name
    }
}
