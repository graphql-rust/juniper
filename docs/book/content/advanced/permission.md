# Guards

Juniper allows to protect object paths with guards. With the context
of the current object, the guard decides if the access is allowed. The
most common case for guards are permission systems. In the following
we explore different permission systems and their usage within
Juniper. Notice that Juniper is not limited to the examples listed
below.

## Role-based

Role based systems consists of roles `R`, users `U`, permissions `P`,
permission mapping `Map: R -> Vec<P>`, and role mapping `Roles: U ->
Vec<R>`. To check if a user has certain permission, the role mapping
is queried and then the permission mapping. We have a user `u` and a
permission `p`, then the permission check is `has: (u, p) -> bool`
which translates to `Roles(U).any(|role| Map(r).contains(p))`.

```rust
struct Database<U, R, P> {
    roles: HashMap<U, Vec<R>>,
    mapping: HashMap<R, Vec<P>>,
}

impl<U, R, P> Database<U, R, P>
where
    R: Eq + std::hash::Hash,
    U: Eq + std::hash::Hash,
    P: PartialEq,
{
    pub fn new(roles: HashMap<U, Vec<R>>, mapping: HashMap<R, Vec<P>>) -> Self {
        Self { roles, mapping }
    }

    pub fn has(&self, user: &U, permission: &P) -> bool {
        self.roles
            .get(user)
            .map(|roles| {
                roles.iter().any(|role| {
                    self.mapping
                        .get(role)
                        .map(|permissions| permissions.contains(permission))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }
}
```

Now we have a simple role-base permission system. The next step is to
implement guard functionality for our role system. To implement this
we must consider what we are protecting. In this case we want to make
sure that only users with the required permissions can access the
path.  We define a contains which holds the information which
permission is required. The contains is evaluated by the guard with
the context of our previously defined database.

```rust
#[derive(Debug, Clone)]
enum AuthResult<U, P> {
    MissingAuthentication,
    WrongPermission(U, P),
}

impl<S, U, P> juniper::IntoFieldError<S> for AuthResult<U, P>
where
    U: std::fmt::Debug,
    P: std::fmt::Debug,
    S: juniper::ScalarValue,
{
    fn into_field_error(self) -> juniper::FieldError<S> {
        match self {
            AuthResult::MissingAuthentication => {
                let v: Value<S> = graphql_value!({
                    "type": "AUTHENTICATION"
                });
                FieldError::new("Not Found", v)
            }
            AuthResult::WrongPermission(user, perm) => {
                let v: Value<S> = graphql_value!({
                    "type": "PERMISSION",
                    "user": { format!("{:?}", user) },
                    "permission": { format!("{:?}", perm) },
                });
                FieldError::new("Not Found", v)
            }
        }
    }
}

struct Context<U, R, P> {
    database: std::sync::Arc<Database<U, R, P>>,
    user: Option<U>,
}

impl<U, R, P> juniper::Context for Context<U, R, P> {}

#[derive(Debug, Clone)]
struct HasPermission<P>(pub P);

impl<S, U, R, P> juniper::GraphQLGuard<S, Context<U, R, P>> for HasPermission<P>
where
    S: juniper::ScalarValue,
    P: Clone + std::fmt::Debug + Send + Sync + PartialEq + 'static,
    U: Clone + std::fmt::Debug + Send + Sync + Eq + std::hash::Hash + 'static,
    R: Eq + std::hash::Hash + Send + Sync + 'static,
{
    type Error = AuthResult<U, P>;

    type CtxOut = Context<U, R, P>;

    fn protected<'a>(
        &'a self,
        ctx: MaybeOwned<'a, Context<U, R, P>>,
    ) -> juniper::BoxFuture<Result<MaybeOwned<'a, Self::CtxOut>, Self::Error>> {
        let val = match ctx.as_ref().user {
            Some(ref user) if ctx.as_ref().database.has(&user, &self.0) => Ok(ctx),
            Some(ref user) => Err(AuthResult::WrongPermission(user.clone(), self.0.clone())),
            None => Err(AuthResult::MissingAuthentication),
        };
        futures::future::FutureExt::boxed(futures::future::ready(val))
    }
}
```

Now we have a generic role-based permission system. The next step is to define our queries and protect them.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
enum Permissions {
    ReadSecret,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum Roles {
    Admin,
    User,
}

struct Query;

#[juniper::graphql_object(Context = Ctx)]
impl Query {
    async fn public(&self, ctx: &Ctx) -> String {
        "Hello World!".to_string()
    }

    #[graphql(Guard = "HasPermission(Permissions::ReadSecret)")]
    async fn private(&self, ctx: &Ctx) -> String {
        "Top secret".to_string()
    }
}
```

Now we have a protected our paths within the query. It is also
possible to protect mutation.

## Generic Context

Sometimes we want to define our guards as general as possible. So
instead of insist of a special context type, we only require a trait
which allows to query the context for the special value which is
required to evaluate our guard. For example, we want to get the
current user. For this purpose we define a type `User`. The trait
`AsRef` from the standard library is used to extract the current user
from the context.

```rust
struct User(String);

struct Context {
    user: Option<User>,
}

impl AsRef<Option<User>> for Context {
    fn as_ref(&self) -> &Option<User> {
        self.user
    }
}
```

To extract the user within out guard, the following is required.

```rust
struct IsLoggedIn;

impl<S, Ctx> juniper::GraphQLGuard<S, Ctx> for IsLoggedIn
where
    S: juniper::ScalarValue + Send + Sync + 'static,
    Ctx: Send + Sync + AsRef<Option<User>> + 'static,
{
    type Error = FieldError<S>;

    type CtxOut = Ctx;

    fn protected<'a>(
        &'a self,
        ctx: &'a Ctx,
    ) -> juniper::BoxFuture<Result<&'a Self::CtxOut, Self::Error>> {
        let val = if AsRef::<Option<User>>::as_ref(ctx).is_some() {
            Ok(ctx)
        } else {
            Err(juniper::FieldError::new("not logged in", juniper::Value::null()))
        };
        futures::future::FutureExt::boxed(futures::future::ready(val))
    }
}
```

Such a design can be useful when dealing with complex contexts.

## Context Overrides

Another interesting feature for permissions systems is the "context
switch". The guard ensures that a user is logged in and maybe has
permissions. Instead of checking the permissions for each following
field, we could change the state. The changed state let the following
fields know that the guard has certain properties. Notice that it is
required to annotate the field with an context. If the field is not
annotated with the desired context, then it is not possible to
determine which argument is the context. Juniper would assume that the
valid context is the object context. However, since we overwritten the
context, we need to annotate it.

```rust
impl juniper::Context for AdvancedContext {}

struct AdvancedContext {
    old: Ctx,
    notice: String,
}

struct Change<P>(pub P, pub String);

impl<S> juniper::GraphQLGuard<S, Ctx> for Change<Permissions>
where
    S: juniper::ScalarValue,
{
    type Error = AuthResult<String, Permissions>;

    type CtxOut = AdvancedContext;

    fn protected<'a>(
        &'a self,
        ctx: MaybeOwned<'a, Ctx>,
    ) -> juniper::BoxFuture<Result<MaybeOwned<'a, Self::CtxOut>, Self::Error>> {
        let val = match ctx.as_ref().user {
            Some(ref user) if ctx.as_ref().database.has(&user, &self.0) => Ok(AdvancedContext {
                old: Clone::clone(ctx.as_ref()),
                notice: self.1.clone(),
            }
            .into()),
            Some(ref user) => Err(AuthResult::WrongPermission(user.clone(), self.0.clone())),
            None => Err(AuthResult::MissingAuthentication),
        };
        futures::future::FutureExt::boxed(futures::future::ready(val))
    }
}
```

The small example uses the context from the previous example and adds
an extra field. In this case the field is taken from the guard itself.

## Top-level guards

TODO: implement this feature

Juniper allows to specify guards for a whole object. In terms of
semantic this is equal to adding the same guard to all fields. If the
field specifies an additional guard, besides the top-level guard, then
both guards are chained. Therefore, first the top-level guard is
executed, afterwards the field-level guard is executed. It is **not**
possible to override the top-level guard.

## Other use-cases

Besides permission checking, guards can be used for other tasks, e.g.,
metric counters. Instead of checking for something, the guard would
increase a counter every time it is evaluated. This is useful for
metric collectors. Notice that the guard is always async, thus HTTP
requests to metrics servers are possible. Overall guards are powerful,
but keep in mind that using them increase the evaluation time.
