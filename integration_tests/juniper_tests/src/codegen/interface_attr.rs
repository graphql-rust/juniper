//! Tests for `#[graphql_interface]` macro.

use juniper::{execute, graphql_object, graphql_value, DefaultScalarValue, EmptyMutation, EmptySubscription, GraphQLObject, GraphQLType, RootNode, ScalarValue, Variables, GraphQLTypeMeta};

/* SUGARED
#[derive(GraphQLObject)]
#[graphql(implements Character)]
struct Human {
    id: String,
    home_planet: String,
}
   DESUGARS INTO: */
#[derive(GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}
#[automatically_derived]
::juniper::inventory::submit! {
    #![crate = juniper]
    ::juniper::GraphQLInterfaceTypeImplementor {
        interface_name: "Character",
        object: ::juniper::GraphQLObjectTypeInfo {
            name: "Human",
            mark_fn: <Human as ::juniper::marker::GraphQLObjectType<::juniper::DefaultScalarValue>>::mark,
        }
    }
}
impl<__S: ::juniper::ScalarValue> ::juniper::AsDynGraphQLType<__S> for Human {
    type Context = <Self as ::juniper::GraphQLType<__S>>::Context;
    type TypeInfo = <Self as ::juniper::GraphQLType<__S>>::TypeInfo;

    fn as_dyn_graphql_type(&self) -> &(dyn GraphQLType<__S, Context = Self::Context, TypeInfo = Self::TypeInfo> + 'static + Send + Sync) {
        self
    }
}

/* SUGARED
#[graphql_interface]
impl Character for Human {
    fn id(&self) -> &str {
        &self.id
    }
}
   DESUGARS INTO: */
impl<GraphQLScalarValue: ::juniper::ScalarValue> Character<GraphQLScalarValue> for Human {
    fn id(&self) -> &str {
        &self.id
    }
}

// ------------------------------------------

/* SUGARED
#[derive(GraphQLObject)]
#[graphql(implements Character)]
struct Droid {
    id: String,
    primary_function: String,
}
   DESUGARS INTO: */
#[derive(GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}
::juniper::inventory::submit! {
    #![crate = juniper]
    ::juniper::GraphQLInterfaceTypeImplementor {
        interface_name: "Character",
        object: ::juniper::GraphQLObjectTypeInfo {
            name: "Droid",
            mark_fn: <Droid as ::juniper::marker::GraphQLObjectType<::juniper::DefaultScalarValue>>::mark,
            reg_fn:
        }
    }
}
#[automatically_derived]
impl<__S: ::juniper::ScalarValue> ::juniper::AsDynGraphQLType<__S> for Droid {
    type Context = <Self as ::juniper::GraphQLType<__S>>::Context;
    type TypeInfo = <Self as ::juniper::GraphQLType<__S>>::TypeInfo;

    fn as_dyn_graphql_type(&self) -> &(dyn GraphQLType<__S, Context = Self::Context, TypeInfo = Self::TypeInfo> + 'static + Send + Sync) {
        self
    }
}

/* SUGARED
#[graphql_interface]
impl Character for Droid {
    fn id(&self) -> &str {
        &self.id
    }

    fn as_droid(&self) -> Option<&Droid> {
        Some(self)
    }
}
   DESUGARS INTO: */
impl<GraphQLScalarValue: ::juniper::ScalarValue> Character<GraphQLScalarValue> for Droid {
    fn id(&self) -> &str {
        &self.id
    }

    fn as_droid(&self) -> Option<&Droid> {
        Some(self)
    }
}

// ------------------------------------------

/* SUGARED
#[graphql_interface]
trait Character {
    fn id(&self) -> &str;

    #[graphql_interface(downcast)]
    fn as_droid(&self) -> Option<&Droid> { None }
}
   DESUGARS INTO: */
trait Character<GraphQLScalarValue: ::juniper::ScalarValue = ::juniper::DefaultScalarValue>: ::juniper::AsDynGraphQLType<GraphQLScalarValue> {
    fn id(&self) -> &str;

    fn as_droid(&self) -> Option<&Droid> { None }
}
#[automatically_derived]
impl<'__obj> ::juniper::marker::GraphQLInterface for dyn Character<Context = (), TypeInfo = ()> + '__obj + Send + Sync
{
    fn mark() {
        if let Some(objects) = ::juniper::GRAPHQL_IFACE_TYPES.get("Character") {
            for obj in objects {
                (obj.mark_fn)();
            }
        }
    }
}
#[automatically_derived]
impl<'__obj, __S> ::juniper::marker::IsOutputType<__S> for dyn Character<__S, Context = (), TypeInfo = ()> + '__obj + Send + Sync
where
    __S: ::juniper::ScalarValue,
{
    fn mark() {
        if let Some(objects) = ::juniper::GRAPHQL_IFACE_TYPES.get("Character") {
            for obj in objects {
                (obj.mark_fn)();
            }
        }
    }
}
#[automatically_derived]
impl<'__obj, __S> ::juniper::GraphQLType<__S> for dyn Character<__S, Context = (), TypeInfo = ()> + '__obj + Send + Sync
where
    __S: ::juniper::ScalarValue,
{
    type Context = ();
    type TypeInfo = ();
    fn type_name<'__i>(&self, info: &'__i Self::TypeInfo) -> Option<&'__i str> {
        <Self as ::juniper::GraphQLTypeMeta<__S>>::name(info)
    }
    fn resolve_field(
        &self,
        _: &Self::TypeInfo,
        field: &str,
        _: &juniper::Arguments<__S>,
        executor: &juniper::Executor<Self::Context, __S>,
    ) -> juniper::ExecutionResult<__S> {
        match field {
            "id" => {
                let res = self.id();
                ::juniper::IntoResolvable::into(res, executor.context()).and_then(|res| match res {
                    Some((ctx, r)) => executor.replaced_context(ctx).resolve_with_ctx(&(), &r),
                    None => Ok(juniper::Value::null()),
                })
            }
            _ => {
                panic!(
                    "Field {} not found on GraphQL interface {}",
                    field, "Character",
                );
            }
        }
    }

    fn concrete_type_name(&self, context: &Self::Context, info: &Self::TypeInfo) -> String {

        // First, check custom downcaster to be used.
        if ({ Character::as_droid(self) } as ::std::option::Option<&Droid>).is_some() {
            return <Droid as ::juniper::GraphQLTypeMeta<__S>>::name(info)
                .unwrap()
                .to_string();
        }

        // Otherwise, get concrete type name as dyn object.
        self.as_dyn_graphql_type().concrete_type_name(context, info)
    }
    fn resolve_into_type(
        &self,
        ti: &Self::TypeInfo,
        type_name: &str,
        _: Option<&[::juniper::Selection<__S>]>,
        executor: &::juniper::Executor<Self::Context, __S>,
    ) -> ::juniper::ExecutionResult<__S> {
        let context = executor.context();

        // First, check custom downcaster to be used.
        if type_name == (<Droid as ::juniper::GraphQLTypeMeta<__S>>::name(ti)).unwrap() {
            return ::juniper::IntoResolvable::into(
                Character::as_droid(self),
                executor.context(),
            )
                .and_then(|res| match res {
                    Some((ctx, r)) => executor.replaced_context(ctx).resolve_with_ctx(&(), &r),
                    None => Ok(::juniper::Value::null()),
                });
        }

        // Otherwise, resolve inner type as dyn object.
        return ::juniper::IntoResolvable::into(
            self.as_dyn_graphql_type(),
            executor.context(),
        )
            .and_then(|res| match res {
                Some((ctx, r)) => executor.replaced_context(ctx).resolve_with_ctx(&(), &r),
                None => Ok(::juniper::Value::null()),
            });
    }
}
#[automatically_derived]
impl<'__obj, __S> ::juniper::GraphQLTypeMeta<__S> for dyn Character<__S, Context = (), TypeInfo = ()> + '__obj + Send + Sync
where
    __S: ::juniper::ScalarValue,
{
    fn name(_: &Self::TypeInfo) -> Option<&str> {
        Some("Character")
    }
    fn meta<'r>(
        info: &Self::TypeInfo,
        registry: &mut ::juniper::Registry<'r, __S>,
    ) -> ::juniper::meta::MetaType<'r, __S>
    where
        __S: 'r,
    {
        // Ensure custom downcaster type is registered
        let _ = registry.get_type::<&Droid>(info);

        // Ensure all child types are registered
        // TODO: how?
        // TODO: get_type_by_name and iter
        //let _ = registry.get_type::<&Human>(info);

        let fields = vec![
            // TODO: try array
            registry.field_convert::<&str, _, Self::Context>("id", info),
        ];

        registry
            .build_interface_type::<dyn Character<__S, Context = (), TypeInfo = ()> + '__obj + Send + Sync>(info, &fields)
            .into_meta()
    }
}
#[automatically_derived]
impl<'__obj, __S> ::juniper::GraphQLTypeAsync<__S> for dyn Character<__S, Context = (), TypeInfo = ()> + '__obj + Send + Sync
where
    __S: ::juniper::ScalarValue,
    Self: Send + Sync,
    __S: Send + Sync,
{
    fn resolve_field_async<'b>(
        &'b self,
        info: &'b Self::TypeInfo,
        field_name: &'b str,
        arguments: &'b ::juniper::Arguments<__S>,
        executor: &'b ::juniper::Executor<Self::Context, __S>,
    ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<__S>> {
        // TODO: similar to what happens in GraphQLType impl
        let res = self.resolve_field(info, field_name, arguments, executor);
        ::juniper::futures::future::FutureExt::boxed(async move { res })
    }

    fn resolve_into_type_async<'b>(
        &'b self,
        ti: &'b Self::TypeInfo,
        type_name: &str,
        se: Option<&'b [::juniper::Selection<'b, __S>]>,
        executor: &'b ::juniper::Executor<'b, 'b, Self::Context, __S>,
    ) -> ::juniper::BoxFuture<'b, ::juniper::ExecutionResult<__S>> {
        // TODO: similar to what happens in GraphQLType impl
        let res = self.resolve_into_type(ti, type_name, se, executor);
        ::juniper::futures::future::FutureExt::boxed(async move { res })
    }
}

// ------------------------------------------

fn schema<'q, C, S, Q>(query_root: Q) -> RootNode<'q, Q, EmptyMutation<C>, EmptySubscription<C>, S>
    where
        Q: GraphQLTypeMeta<S, Context = C, TypeInfo = ()> + 'q,
        S: ScalarValue + 'q,
{
    RootNode::new(
        query_root,
        EmptyMutation::<C>::new(),
        EmptySubscription::<C>::new(),
    )
}

mod poc {
    use super::*;

    type DynCharacter<'a, S = DefaultScalarValue> = dyn Character<S, Context=(), TypeInfo=()> + 'a + Send + Sync;

    enum QueryRoot {
        Human,
        Droid,
    }

    #[graphql_object]
    impl QueryRoot {
        fn character(&self) -> Box<DynCharacter<'_>> {
            let ch: Box<DynCharacter<'_>> = match self {
                Self::Human => Box::new(Human {
                    id: "human-32".to_string(),
                    home_planet: "earth".to_string(),
                }),
                Self::Droid => Box::new(Droid {
                    id: "droid-99".to_string(),
                    primary_function: "run".to_string(),
                }),
            };
            ch
        }
    }

    #[tokio::test]
    async fn resolves_id_for_human() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"id": "human-32"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_id_for_droid() {
        const DOC: &str = r#"{
            character {
                id
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"id": "droid-99"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_human() {
        const DOC: &str = r#"{
            character {
                ... on Human {
                    humanId: id
                    homePlanet
                }
            }
        }"#;

        let schema = schema(QueryRoot::Human);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"humanId": "human-32", "homePlanet": "earth"}}),
                vec![],
            )),
        );
    }

    #[tokio::test]
    async fn resolves_droid() {
        const DOC: &str = r#"{
            character {
                ... on Droid {
                    humanId: id
                    primaryFunction
                }
            }
        }"#;

        let schema = schema(QueryRoot::Droid);

        assert_eq!(
            execute(DOC, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"character": {"droidId": "droid-99", "primaryFunction": "run"}}),
                vec![],
            )),
        );
    }
}