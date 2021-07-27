pub use crate::tracing::{
    async_tokens, span_tokens, sync_tokens, Attr, FieldBehaviour, Rule, TracedArgument,
    TracedField, TracedType,
};

use super::{Definition, Field, FieldArgument};

impl TracedType for Definition {
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

impl TracedField for Field {
    type Arg = FieldArgument;

    fn instrument(&self) -> Option<&Attr> {
        self.instrument.as_ref()
    }

    fn tracing_behaviour(&self) -> FieldBehaviour {
        self.tracing.unwrap_or(FieldBehaviour::Default)
    }

    fn is_async(&self) -> bool {
        self.is_async
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn args(&self) -> Vec<&Self::Arg> {
        self.arguments
            .iter()
            .filter_map(|arg| arg.as_regular())
            .collect()
    }
}

impl TracedArgument for FieldArgument {
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
