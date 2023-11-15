//! GraphQL support for [`anyhow::Error`].

use crate::{FieldError, IntoFieldError, ScalarValue, Value};

impl<S: ScalarValue> IntoFieldError<S> for anyhow::Error {
    fn into_field_error(self) -> FieldError<S> {
        #[cfg(any(nightly, feature = "backtrace"))]
        let extensions = {
            let backtrace = self.backtrace().to_string();
            if backtrace == "disabled backtrace" {
                Value::Null
            } else {
                let mut obj = crate::value::Object::with_capacity(1);
                _ = obj.add_field(
                    "backtrace",
                    Value::List(
                        backtrace
                            .split('\n')
                            .map(|line| Value::Scalar(line.to_owned().into()))
                            .collect(),
                    ),
                );
                Value::Object(obj)
            }
        };
        #[cfg(not(any(nightly, feature = "backtrace")))]
        let extensions = Value::Null;

        FieldError::new(self, extensions)
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use anyhow::anyhow;
    use serial_test::serial;

    use crate::{
        execute, graphql_object, graphql_value, graphql_vars, parser::SourcePosition,
        EmptyMutation, EmptySubscription, RootNode,
    };

    #[tokio::test]
    #[serial]
    async fn simple() {
        struct Root;

        #[graphql_object]
        impl Root {
            fn err() -> anyhow::Result<i32> {
                Err(anyhow!("errored!"))
            }
        }

        let prev_env = env::var("RUST_BACKTRACE").ok();
        env::set_var("RUST_BACKTRACE", "1");

        const DOC: &str = r#"{
            err
        }"#;

        let schema = RootNode::new(
            Root,
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );

        let res = execute(DOC, None, &schema, &graphql_vars! {}, &()).await;

        assert!(res.is_ok(), "failed: {:?}", res.unwrap_err());

        let (val, errs) = res.unwrap();

        assert_eq!(val, graphql_value!(null));
        assert_eq!(errs.len(), 1, "too many errors: {errs:?}");

        let err = errs.first().unwrap();

        assert_eq!(*err.location(), SourcePosition::new(14, 1, 12));
        assert_eq!(err.path(), &["err"]);

        let err = err.error();

        assert_eq!(err.message(), "errored!");
        #[cfg(not(any(nightly, feature = "backtrace")))]
        assert_eq!(err.extensions(), &graphql_value!(null));
        #[cfg(any(nightly, feature = "backtrace"))]
        assert_eq!(
            err.extensions()
                .as_object_value()
                .map(|ext| ext.contains_field("backtrace")),
            Some(true),
            "no `backtrace` in extensions: {err:?}",
        );

        if let Some(val) = prev_env {
            env::set_var("RUST_BACKTRACE", val);
        }
    }
}
