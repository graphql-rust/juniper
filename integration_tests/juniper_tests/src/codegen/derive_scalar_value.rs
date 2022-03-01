use juniper::{DefaultScalarValue, GraphQLScalarValue, ScalarValue as _};
use serde::{Deserialize, Serialize};

mod trivial {
    use super::*;

    #[derive(Clone, Debug, Deserialize, GraphQLScalarValue, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        #[graphql(as_int, as_float)]
        Int(i32),
        #[graphql(as_float)]
        Float(f64),
        #[graphql(as_str, as_string, into_string)]
        String(String),
        #[graphql(as_boolean)]
        Boolean(bool),
    }

    #[test]
    fn into_another() {
        assert!(CustomScalarValue::from(5)
            .into_another::<DefaultScalarValue>()
            .is_type::<i32>());
        assert!(CustomScalarValue::from(0.5_f64)
            .into_another::<DefaultScalarValue>()
            .is_type::<f64>());
        assert!(CustomScalarValue::from("str".to_owned())
            .into_another::<DefaultScalarValue>()
            .is_type::<String>());
        assert!(CustomScalarValue::from(true)
            .into_another::<DefaultScalarValue>()
            .is_type::<bool>());
    }
}

mod named_fields {
    use super::*;

    #[derive(Clone, Debug, Deserialize, GraphQLScalarValue, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        #[graphql(as_int, as_float)]
        Int { int: i32 },
        #[graphql(as_float)]
        Float(f64),
        #[graphql(as_str, as_string, into_string)]
        String(String),
        #[graphql(as_boolean)]
        Boolean { v: bool },
    }

    #[test]
    fn into_another() {
        assert!(CustomScalarValue::from(5)
            .into_another::<DefaultScalarValue>()
            .is_type::<i32>());
        assert!(CustomScalarValue::from(0.5_f64)
            .into_another::<DefaultScalarValue>()
            .is_type::<f64>());
        assert!(CustomScalarValue::from("str".to_owned())
            .into_another::<DefaultScalarValue>()
            .is_type::<String>());
        assert!(CustomScalarValue::from(true)
            .into_another::<DefaultScalarValue>()
            .is_type::<bool>());
    }
}

mod custom_fn {
    use super::*;

    #[derive(Clone, Debug, Deserialize, GraphQLScalarValue, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        #[graphql(as_int, as_float)]
        Int(i32),
        #[graphql(as_float)]
        Float(f64),
        #[graphql(
            as_str,
            as_string = str::to_owned,
            into_string = std::convert::identity,
        )]
        String(String),
        #[graphql(as_boolean)]
        Boolean(bool),
    }

    #[test]
    fn into_another() {
        assert!(CustomScalarValue::from(5)
            .into_another::<DefaultScalarValue>()
            .is_type::<i32>());
        assert!(CustomScalarValue::from(0.5_f64)
            .into_another::<DefaultScalarValue>()
            .is_type::<f64>());
        assert!(CustomScalarValue::from("str".to_owned())
            .into_another::<DefaultScalarValue>()
            .is_type::<String>());
        assert!(CustomScalarValue::from(true)
            .into_another::<DefaultScalarValue>()
            .is_type::<bool>());
    }
}

mod allow_missing_attributes {
    use super::*;

    #[derive(Clone, Debug, Deserialize, GraphQLScalarValue, PartialEq, Serialize)]
    #[graphql(allow_missing_attributes)]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        Int(i32),
        #[graphql(as_float)]
        Float(f64),
        #[graphql(as_str, as_string, into_string)]
        String(String),
        #[graphql(as_boolean)]
        Boolean(bool),
    }

    #[test]
    fn into_another() {
        assert!(CustomScalarValue::Int(5).as_int().is_none());
        assert!(CustomScalarValue::from(0.5_f64)
            .into_another::<DefaultScalarValue>()
            .is_type::<f64>());
        assert!(CustomScalarValue::from("str".to_owned())
            .into_another::<DefaultScalarValue>()
            .is_type::<String>());
        assert!(CustomScalarValue::from(true)
            .into_another::<DefaultScalarValue>()
            .is_type::<bool>());
    }
}
