//! GraphQL engines under benchmark.

pub mod apollo_compiler;
pub mod async_graphql;
pub mod juniper;

/// GraphQL engine that serves the schema of this crate.
pub trait Engine: Default {
    /// Name of the engine in benchmark IDs.
    const NAME: &'static str;

    /// The engine supports the `isOneOf` field of `__Type`.
    ///
    /// The September 2025 GraphQL specification added this field.
    const SUPPORTS_IS_ONE_OF: bool = true;

    /// Executes a GraphQL-over-HTTP JSON `request` body and returns the JSON response body.
    ///
    /// # Panics
    ///
    /// If `request` is not a valid request body.
    fn execute(&self, request: &str) -> String;

    /// Executes `request` with [`Engine::execute()`] and parses the response.
    ///
    /// # Panics
    ///
    /// If `request` is not a valid request body, or if the response contains errors.
    fn response(&self, request: &str) -> serde_json::Value {
        let response: serde_json::Value =
            serde_json::from_str(&self.execute(request)).expect("response is valid JSON");
        assert!(
            response.get("errors").is_none(),
            "`{}` returned errors: {response}",
            Self::NAME,
        );
        response
    }
}
