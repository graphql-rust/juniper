//!
use crate::{
    guards::MaybeOwned, EmptyMutation, EmptySubscription, FieldError, GraphQLGuardExt, RootNode,
};
use std::{collections::HashMap, sync::Arc};

pub struct IdentityGuard;

impl<CtxIn> crate::GraphQLGuard<crate::DefaultScalarValue, CtxIn> for IdentityGuard
where
    CtxIn: Send + Sync + 'static,
{
    type Error = crate::FieldError<crate::DefaultScalarValue>;

    type CtxOut = CtxIn;

    /// Protects a GraphQL path resource.
    fn protected<'a>(
        &'a self,
        ctx: MaybeOwned<'a, CtxIn>,
    ) -> crate::BoxFuture<Result<MaybeOwned<'a, Self::CtxOut>, Self::Error>> {
        futures::future::FutureExt::boxed(futures::future::ready(Ok(ctx)))
    }
}

pub struct RoleGuard(pub &'static str);

impl crate::GraphQLGuard<crate::DefaultScalarValue, Ctx> for RoleGuard {
    type Error = crate::FieldError<crate::DefaultScalarValue>;

    type CtxOut = Ctx;

    /// Protects a GraphQL path resource.
    fn protected<'a>(
        &'a self,
        ctx: MaybeOwned<'a, Ctx>,
    ) -> crate::BoxFuture<Result<MaybeOwned<'a, Self::CtxOut>, Self::Error>> {
        let f = async move {
            match ctx.as_ref().active {
                Some(ref active_user) => match ctx.as_ref().users.get(active_user) {
                    Some(groups) if groups.iter().find(|val| val.as_str() == self.0).is_some() => {
                        Ok(ctx)
                    }
                    Some(_) => Err(FieldError::new(
                        "User is not part of group",
                        graphql_value!({ "group": { self.0.to_string() } }),
                    )),
                    None => Err(FieldError::new(
                        "Unknown user",
                        graphql_value!({ "user": { active_user.to_string() } }),
                    )),
                },
                None => Err(FieldError::new(
                    "Missing authentication",
                    crate::Value::Null,
                )),
            }
        };

        futures::future::FutureExt::boxed(f)
    }
}

pub struct Ctx {
    users: Arc<HashMap<String, Vec<String>>>,
    active: Option<String>,
}

impl Ctx {
    fn mapping() -> HashMap<String, Vec<String>> {
        let mut map = HashMap::new();

        map.insert(
            "user_a".to_string(),
            vec!["USER", "PREMIUM"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        );

        map.insert(
            "user_b".to_string(),
            vec!["USER", "ADMIN"]
                .into_iter()
                .map(str::to_string)
                .collect(),
        );

        map
    }

    pub fn none() -> Self {
        Ctx {
            users: Arc::new(Self::mapping()),
            active: None,
        }
    }

    pub fn user_a() -> Self {
        Ctx {
            users: Arc::new(Self::mapping()),
            active: Some("user_a".to_string()),
        }
    }

    pub fn user_b() -> Self {
        Ctx {
            users: Arc::new(Self::mapping()),
            active: Some("user_b".to_string()),
        }
    }
}

impl crate::Context for Ctx {}

pub struct Query;

#[crate::graphql_object_internal(Context = Ctx)]
impl Query {
    #[graphql(Guard = "IdentityGuard")]
    async fn public() -> i32 {
        0
    }

    #[graphql(Guard = "RoleGuard(\"ADMIN\")")]
    async fn private() -> i32 {
        0
    }

    // FIXME: does not work
    #[graphql(Guard = "IdentityGuard.and_then(RoleGuard(\"ADMIN\"))")]
    async fn combined() -> i32 {
        0
    }
}

#[tokio::test]
async fn guard_user_a_private() {
    let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
    let doc = r#"
        query {
            private
        }
    "#;

    let vars = Default::default();
    let ctx = Ctx::user_a();
    let (_res, errs) = crate::execute(doc, None, &schema, &vars, &ctx)
        .await
        .unwrap();

    assert!(!errs.is_empty());
}

#[tokio::test]
async fn guard_user_a_public() {
    let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
    let doc = r#"
        query {
            public
        }
    "#;

    let vars = Default::default();
    let ctx = Ctx::user_a();
    let (_res, errs) = crate::execute(doc, None, &schema, &vars, &ctx)
        .await
        .unwrap();

    assert!(errs.is_empty());
}

#[tokio::test]
async fn guard_user_b_private() {
    let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
    let doc = r#"
        query {
            private
        }
    "#;

    let vars = Default::default();
    let ctx = Ctx::user_b();
    let (_res, errs) = crate::execute(doc, None, &schema, &vars, &ctx)
        .await
        .unwrap();

    assert!(errs.is_empty());
}

#[tokio::test]
async fn guard_user_b_public() {
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Ctx>::new(),
        EmptySubscription::<Ctx>::new(),
    );
    let doc = r#"
        query {
            public
        }
    "#;

    let vars = Default::default();
    let ctx = Ctx::user_b();
    let (_res, errs) = crate::execute(doc, None, &schema, &vars, &ctx)
        .await
        .unwrap();

    assert!(errs.is_empty());
}

#[tokio::test]
async fn guard_none_public() {
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Ctx>::new(),
        EmptySubscription::<Ctx>::new(),
    );
    let doc = r#"
        query {
            public
        }
    "#;

    let vars = Default::default();
    let ctx = Ctx::user_b();
    let (_res, errs) = crate::execute(doc, None, &schema, &vars, &ctx)
        .await
        .unwrap();

    assert!(errs.is_empty());
}

#[tokio::test]
async fn guard_none_private() {
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Ctx>::new(),
        EmptySubscription::<Ctx>::new(),
    );
    let doc = r#"
        query {
            private
        }
    "#;

    let vars = Default::default();
    let ctx = Ctx::none();
    let (_res, errs) = crate::execute(doc, None, &schema, &vars, &ctx)
        .await
        .unwrap();

    assert!(!errs.is_empty());
}

//FIXME: re-enable
// #[tokio::test]
// async fn guard_none_combined() {
//     let schema = RootNode::new(
//         Query,
//         EmptyMutation::<Ctx>::new(),
//         EmptySubscription::<Ctx>::new(),
//     );
//     let doc = r#"
//         query {
//             combined
//         }
//     "#;

//     let vars = Default::default();
//     let ctx = Ctx::none();
//     let (_res, errs) = crate::execute(doc, None, &schema, &vars, &ctx)
//         .await
//         .unwrap();

//     assert!(!errs.is_empty());
// }
