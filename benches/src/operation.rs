//! Operations run by the benchmarks.

use crate::engine::{
    Engine,
    apollo_compiler::ApolloCompiler,
    async_graphql::AsyncGraphql,
    juniper::{JuniperAsync, JuniperAsyncFn, JuniperSync},
};

/// GraphQL operation with its variables.
#[derive(Clone, Copy, Debug)]
pub struct Operation {
    /// Name of the operation in benchmark IDs.
    pub name: &'static str,

    /// GraphQL document with a single operation.
    pub document: &'static str,

    /// Variables of the operation as a JSON object.
    pub variables: &'static str,

    /// All engines return the same response.
    ///
    /// Introspection responses differ because each engine adds its own built-in types and
    /// directives to the schema.
    pub same_response: bool,

    /// The operation selects the `isOneOf` field of `__Type`.
    ///
    /// The September 2025 specification added this field. Only engines with
    /// [`Engine::SUPPORTS_IS_ONE_OF`] run the operation.
    pub selects_is_one_of: bool,
}

impl Operation {
    /// Returns the GraphQL-over-HTTP JSON request body of this operation.
    ///
    /// # Panics
    ///
    /// If [`Operation::variables`] is not valid JSON.
    pub fn request(&self) -> String {
        let variables: serde_json::Value =
            serde_json::from_str(self.variables).expect("variables are valid JSON");
        serde_json::json!({ "query": self.document, "variables": variables }).to_string()
    }

    /// Indicates whether the `E` engine supports all features that this operation uses.
    pub fn runs_on<E: Engine>(&self) -> bool {
        !self.selects_is_one_of || E::SUPPORTS_IS_ONE_OF
    }

    /// Executes this operation once with every engine that supports it, and checks the
    /// responses.
    ///
    /// The `juniper` engines serve one schema, so they always return the same response.
    ///
    /// # Panics
    ///
    /// If a response contains errors, or if the engines return different responses for an
    /// operation with [`Operation::same_response`].
    pub fn check(&self) {
        let request = self.request();
        let juniper = JuniperAsync::default().response(&request);
        assert_eq!(
            JuniperSync::default().response(&request),
            juniper,
            "synchronous and asynchronous `juniper` responses differ for `{}`",
            self.name,
        );
        assert_eq!(
            JuniperAsyncFn::default().response(&request),
            juniper,
            "`juniper` responses with `async fn` and synchronous resolvers differ for `{}`",
            self.name,
        );
        self.check_engine::<AsyncGraphql>(&request, &juniper);
        self.check_engine::<ApolloCompiler>(&request, &juniper);
    }

    /// Executes this operation once with the `E` engine, if the engine supports it, and checks
    /// the response against the `juniper` response.
    fn check_engine<E: Engine>(&self, request: &str, juniper: &serde_json::Value) {
        if !self.runs_on::<E>() {
            return;
        }
        let response = E::default().response(request);
        if self.same_response {
            assert_eq!(
                &response,
                juniper,
                "`{}` and `juniper` responses differ for `{}`",
                E::NAME,
                self.name,
            );
        }
    }
}

/// Single scalar field. Measures the fixed cost of a request.
pub const MINIMAL: Operation = Operation {
    name: "minimal",
    document: include_str!("operation/minimal.graphql"),
    variables: "{}",
    same_response: true,
    selects_is_one_of: false,
};

/// List of 1000 objects with 6 scalar fields each. Measures the cost of a field.
pub const FLAT_LIST: Operation = Operation {
    name: "flat_list",
    document: include_str!("operation/flat_list.graphql"),
    variables: "{}",
    same_response: true,
    selects_is_one_of: false,
};

/// Three levels of lists with 2220 objects in total. Measures the cost of a nested object.
pub const NESTED_LISTS: Operation = Operation {
    name: "nested_lists",
    document: include_str!("operation/nested_lists.graphql"),
    variables: "{}",
    same_response: true,
    selects_is_one_of: false,
};

/// Query of the `async-graphql` benchmark with plain lists instead of connections.
pub const STAR_WARS: Operation = Operation {
    name: "star_wars",
    document: include_str!("operation/star_wars.graphql"),
    variables: "{}",
    same_response: true,
    selects_is_one_of: false,
};

/// Fragments on an interface, variables, and directives.
pub const FRAGMENTS: Operation = Operation {
    name: "fragments",
    document: include_str!("operation/fragments.graphql"),
    variables: r#"{"episode": "EMPIRE", "withFriends": true}"#,
    same_response: true,
    selects_is_one_of: false,
};

/// Introspection query of GraphiQL.
pub const INTROSPECTION: Operation = Operation {
    name: "introspection",
    document: include_str!("operation/introspection.graphql"),
    variables: "{}",
    same_response: false,
    selects_is_one_of: true,
};

/// Introspection query of GraphiQL without the `isOneOf` field.
///
/// Engines without the `isOneOf` field run this operation instead of [`INTROSPECTION`].
pub const INTROSPECTION_WITHOUT_ONE_OF: Operation = Operation {
    name: "introspection_without_one_of",
    document: include_str!("operation/introspection_without_one_of.graphql"),
    variables: "{}",
    same_response: false,
    selects_is_one_of: false,
};

/// All operations in benchmark order.
pub const ALL: [Operation; 7] = [
    MINIMAL,
    FLAT_LIST,
    NESTED_LISTS,
    STAR_WARS,
    FRAGMENTS,
    INTROSPECTION,
    INTROSPECTION_WITHOUT_ONE_OF,
];

#[cfg(test)]
mod tests {
    use super::{ALL, INTROSPECTION, INTROSPECTION_WITHOUT_ONE_OF, Operation};
    use crate::engine::{apollo_compiler::ApolloCompiler, juniper::JuniperAsync};

    #[test]
    fn engines_agree() {
        for operation in ALL {
            operation.check();
        }
    }

    #[test]
    #[should_panic(expected = "returned errors")]
    fn check_rejects_errors() {
        Operation {
            name: "unknown_field",
            document: "{ unknown }",
            variables: "{}",
            same_response: true,
            selects_is_one_of: false,
        }
        .check();
    }

    mod runs_on {
        use super::{ApolloCompiler, INTROSPECTION, INTROSPECTION_WITHOUT_ONE_OF, JuniperAsync};

        #[test]
        fn engine_with_is_one_of_runs_both_introspections() {
            assert!(INTROSPECTION.runs_on::<JuniperAsync>());
            assert!(INTROSPECTION_WITHOUT_ONE_OF.runs_on::<JuniperAsync>());
        }

        #[test]
        fn engine_without_is_one_of_skips_introspection_with_it() {
            assert!(!INTROSPECTION.runs_on::<ApolloCompiler>());
            assert!(INTROSPECTION_WITHOUT_ONE_OF.runs_on::<ApolloCompiler>());
        }
    }
}
