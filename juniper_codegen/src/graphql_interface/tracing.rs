pub use crate::tracing::{
    async_tokens, instrument, span_tokens, sync_tokens, Attr, Rule, TracedArgument, TracedField,
    TracedType,
};

use super::{Definition, Field, FieldArgument};

impl TracedType for Definition {
    fn tracing_rule(&self) -> Option<Rule> {
        self.tracing_rule
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

    fn tracing_attr(&self) -> Option<&Attr> {
        self.tracing.as_ref()
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
