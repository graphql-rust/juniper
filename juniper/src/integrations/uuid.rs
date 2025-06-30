//! GraphQL support for [`uuid`] crate types.
//!
//! # Supported types
//!
//! | Rust type | GraphQL scalar |
//! |-----------|----------------|
//! | [`Uuid`]  | [`UUID`][s1]   |
//!
//! [`Uuid`]: uuid::Uuid
//! [s1]: https://graphql-scalars.dev/docs/scalars/uuid

use crate::{ScalarValue, graphql_scalar};

/// [Universally Unique Identifier][0] (UUID).
///
/// [`UUID` scalar][1] compliant.
///
/// See also [`uuid::Uuid`][2] for details.
///
/// [0]: https://en.wikipedia.org/wiki/Universally_unique_identifier
/// [1]: https://graphql-scalars.dev/docs/scalars/uuid
/// [2]: https://docs.rs/uuid/*/uuid/struct.Uuid.html
#[graphql_scalar]
#[graphql(
    name = "UUID",
    with = uuid_scalar,
    to_output_with = ScalarValue::from_displayable,
    parse_token(String),
    specified_by_url = "https://graphql-scalars.dev/docs/scalars/uuid",
)]
type Uuid = uuid::Uuid;

mod uuid_scalar {
    use super::Uuid;

    pub(super) fn from_input(s: &str) -> Result<Uuid, Box<str>> {
        Uuid::parse_str(s).map_err(|e| format!("Failed to parse `UUID`: {e}").into())
    }
}

#[cfg(test)]
mod test {
    use uuid::Uuid;

    use crate::{FromInputValue, InputValue, graphql_input_value};

    #[test]
    fn uuid_from_input() {
        let raw = "123e4567-e89b-12d3-a456-426655440000";
        let input: InputValue = graphql_input_value!((raw));

        let parsed: Uuid = FromInputValue::from_input_value(&input).unwrap();
        let id = Uuid::parse_str(raw).unwrap();

        assert_eq!(parsed, id);
    }
}
