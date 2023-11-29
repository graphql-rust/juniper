//! Tests for `#[derive(ScalarValue)]` macro.

pub mod common;

use juniper::{DefaultScalarValue, ScalarValue};
use serde::{Deserialize, Serialize};

// Override `std::prelude` items to check whether macros expand hygienically.
#[allow(unused_imports)]
use self::common::hygiene::*;

mod trivial {
    use super::*;

    #[derive(Clone, Debug, Deserialize, PartialEq, ScalarValue, Serialize)]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        #[value(as_float, as_int)]
        Int(i32),
        #[value(as_float)]
        Float(f64),
        #[value(as_str, as_string, into_string)]
        String(prelude::String),
        #[value(as_bool)]
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
            .is_type::<prelude::String>());
        assert!(CustomScalarValue::from(true)
            .into_another::<DefaultScalarValue>()
            .is_type::<bool>());
    }
}

mod named_fields {
    use super::*;

    #[derive(Clone, Debug, Deserialize, PartialEq, ScalarValue, Serialize)]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        #[value(as_float, as_int)]
        Int { int: i32 },
        #[value(as_float)]
        Float(f64),
        #[value(as_str, as_string, into_string)]
        String(prelude::String),
        #[value(as_bool)]
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
            .is_type::<prelude::String>());
        assert!(CustomScalarValue::from(true)
            .into_another::<DefaultScalarValue>()
            .is_type::<bool>());
    }
}

mod custom_fn {
    use super::*;

    #[derive(Clone, Debug, Deserialize, PartialEq, ScalarValue, Serialize)]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        #[value(as_float, as_int)]
        Int(i32),
        #[value(as_float)]
        Float(f64),
        #[value(
            as_str,
            as_string = str::to_owned,
            into_string = std::convert::identity,
        )]
        String(prelude::String),
        #[value(as_bool)]
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
            .is_type::<prelude::String>());
        assert!(CustomScalarValue::from(true)
            .into_another::<DefaultScalarValue>()
            .is_type::<bool>());
    }
}

mod allow_missing_attributes {
    use super::*;

    #[derive(Clone, Debug, Deserialize, PartialEq, ScalarValue, Serialize)]
    #[serde(untagged)]
    #[value(allow_missing_attributes)]
    pub enum CustomScalarValue {
        Int(i32),
        #[value(as_float)]
        Float(f64),
        #[value(as_str, as_string, into_string)]
        String(prelude::String),
        #[value(as_bool)]
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
            .is_type::<prelude::String>());
        assert!(CustomScalarValue::from(true)
            .into_another::<DefaultScalarValue>()
            .is_type::<bool>());
    }
}
