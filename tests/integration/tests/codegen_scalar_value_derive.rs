//! Tests for `#[derive(ScalarValue)]` macro.

pub mod common;

use derive_more::with_trait::{Display, From, TryInto};
use juniper::{DefaultScalarValue, ScalarValue, TryToPrimitive};
use serde::{Deserialize, Serialize};

// Override `std::prelude` items to check whether macros expand hygienically.
use self::common::hygiene::*;

mod trivial {
    use super::*;

    #[derive(
        Clone, Debug, Deserialize, Display, From, PartialEq, ScalarValue, Serialize, TryInto,
    )]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        #[value(to_float, to_int)]
        Int(i32),
        #[value(to_float)]
        Float(f64),
        #[value(as_str, to_string)]
        String(prelude::String),
        #[value(to_bool)]
        Boolean(bool),
    }

    #[test]
    fn into_another() {
        assert!(
            CustomScalarValue::from(5)
                .into_another::<DefaultScalarValue>()
                .is_type::<i32>()
        );
        assert!(
            CustomScalarValue::from(0.5_f64)
                .into_another::<DefaultScalarValue>()
                .is_type::<f64>()
        );
        assert!(
            CustomScalarValue::from("str".to_owned())
                .into_another::<DefaultScalarValue>()
                .is_type::<prelude::String>()
        );
        assert!(
            CustomScalarValue::from(true)
                .into_another::<DefaultScalarValue>()
                .is_type::<bool>()
        );
    }
}

mod named_fields {
    use super::*;

    #[derive(
        Clone, Debug, Deserialize, Display, From, PartialEq, ScalarValue, Serialize, TryInto,
    )]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        #[value(to_float, to_int)]
        Int { int: i32 },
        #[value(to_float)]
        Float(f64),
        #[value(as_str, to_string)]
        String(prelude::String),
        #[value(to_bool)]
        Boolean { v: bool },
    }

    #[test]
    fn into_another() {
        assert!(
            CustomScalarValue::from(5)
                .into_another::<DefaultScalarValue>()
                .is_type::<i32>()
        );
        assert!(
            CustomScalarValue::from(0.5_f64)
                .into_another::<DefaultScalarValue>()
                .is_type::<f64>()
        );
        assert!(
            CustomScalarValue::from("str".to_owned())
                .into_another::<DefaultScalarValue>()
                .is_type::<prelude::String>()
        );
        assert!(
            CustomScalarValue::from(true)
                .into_another::<DefaultScalarValue>()
                .is_type::<bool>()
        );
    }
}

mod custom_fn {
    use super::*;

    #[derive(
        Clone, Debug, Deserialize, Display, From, PartialEq, ScalarValue, Serialize, TryInto,
    )]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        #[value(to_float, to_int)]
        Int(i32),
        #[value(to_float)]
        Float(f64),
        #[value(
            as_str,
            to_string = str::to_owned,
        )]
        String(prelude::String),
        #[value(to_bool)]
        Boolean(bool),
    }

    #[test]
    fn into_another() {
        assert!(
            CustomScalarValue::from(5)
                .into_another::<DefaultScalarValue>()
                .is_type::<i32>()
        );
        assert!(
            CustomScalarValue::from(0.5_f64)
                .into_another::<DefaultScalarValue>()
                .is_type::<f64>()
        );
        assert!(
            CustomScalarValue::from("str".to_owned())
                .into_another::<DefaultScalarValue>()
                .is_type::<prelude::String>()
        );
        assert!(
            CustomScalarValue::from(true)
                .into_another::<DefaultScalarValue>()
                .is_type::<bool>()
        );
    }
}

mod missing_conv_attr {
    use super::*;

    #[derive(
        Clone, Debug, Deserialize, Display, From, PartialEq, ScalarValue, Serialize, TryInto,
    )]
    #[serde(untagged)]
    pub enum CustomScalarValue {
        Int(i32),
        #[value(to_float)]
        Float(f64),
        #[value(as_str, to_string)]
        String(prelude::String),
        #[value(to_bool)]
        Boolean(bool),
    }

    impl<'me> TryToPrimitive<'me, i32> for CustomScalarValue {
        type Error = &'static str;

        fn try_to_primitive(&'me self) -> prelude::Result<i32, Self::Error> {
            Err("Not `Int` definitely")
        }
    }

    #[test]
    fn into_another() {
        assert!(CustomScalarValue::Int(5).try_to_int().is_none());
        assert!(
            CustomScalarValue::from(0.5_f64)
                .into_another::<DefaultScalarValue>()
                .is_type::<f64>()
        );
        assert!(
            CustomScalarValue::from("str".to_owned())
                .into_another::<DefaultScalarValue>()
                .is_type::<prelude::String>()
        );
        assert!(
            CustomScalarValue::from(true)
                .into_another::<DefaultScalarValue>()
                .is_type::<bool>()
        );
    }
}
