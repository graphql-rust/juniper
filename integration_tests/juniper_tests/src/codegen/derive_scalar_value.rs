use juniper::{DefaultScalarValue, ScalarValue};
use serde::{Deserialize, Serialize};

mod trivial {
    use super::*;

    #[derive(Clone, Debug, Deserialize, ScalarValue, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        #[scalar_value(as_int, as_float)]
        Int(i32),
        #[scalar_value(as_float)]
        Float(f64),
        #[scalar_value(as_str, as_string, into_string)]
        String(String),
        #[scalar_value(as_bool)]
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

    #[derive(Clone, Debug, Deserialize, ScalarValue, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        #[scalar_value(as_int, as_float)]
        Int { int: i32 },
        #[scalar_value(as_float)]
        Float(f64),
        #[scalar_value(as_str, as_string, into_string)]
        String(String),
        #[scalar_value(as_bool)]
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

    #[derive(Clone, Debug, Deserialize, ScalarValue, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        #[scalar_value(as_int, as_float)]
        Int(i32),
        #[scalar_value(as_float)]
        Float(f64),
        #[scalar_value(
            as_str,
            as_string = str::to_owned,
            into_string = std::convert::identity,
        )]
        String(String),
        #[scalar_value(as_bool)]
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

    #[derive(Clone, Debug, Deserialize, ScalarValue, PartialEq, Serialize)]
    #[serde(untagged)]
    #[scalar_value(allow_missing_attributes)]
    pub enum CustomScalarValue {
        Int(i32),
        #[scalar_value(as_float)]
        Float(f64),
        #[scalar_value(as_str, as_string, into_string)]
        String(String),
        #[scalar_value(as_bool)]
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
