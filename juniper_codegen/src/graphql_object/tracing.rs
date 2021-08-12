pub use crate::tracing::*;

use super::Definition;
use syn::Type;

impl<T: ?Sized> TracedType for Definition<T> {
    fn tracing_rule(&self) -> Rule {
        self.tracing.unwrap_or(Rule::All)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn scalar(&self) -> Option<Type> {
        Some(self.scalar.ty())
    }
}
