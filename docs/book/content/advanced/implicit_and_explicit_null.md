# Implicit and explicit null

There are two ways that a client can submit a null argument or field in a query.

They can use a null literal:

```graphql
{
    field(arg: null)
}
```

Or they can simply omit the argument:

```graphql
{
    field
}
```

The former is an explicit null and the latter is an implicit null.

There are some situations where it's useful to know which one the user provided.

For example, let's say your business logic has a function that allows users to
perform a "patch" operation on themselves. Let's say your users can optionally
have favorite and least favorite numbers, and the input for that might look
like this:

```rust
/// Updates user attributes. Fields that are `None` are left as-is.
pub struct UserPatch {
    /// If `Some`, updates the user's favorite number.
    pub favorite_number: Option<Option<i32>>,

    /// If `Some`, updates the user's least favorite number.
    pub least_favorite_number: Option<Option<i32>>,
}

# fn main() {}
```

To set a user's favorite number to 7, you would set `favorite_number` to
`Some(Some(7))`. In GraphQL, that might look like this:

```graphql
mutation { patchUser(patch: { favoriteNumber: 7 }) }
```

To unset the user's favorite number, you would set `favorite_number` to
`Some(None)`. In GraphQL, that might look like this:

```graphql
mutation { patchUser(patch: { favoriteNumber: null }) }
```

If you want to leave the user's favorite number alone, you would set it to
`None`. In GraphQL, that might look like this:

```graphql
mutation { patchUser(patch: {}) }
```

The last two cases rely on being able to distinguish between explicit and implicit null.

In Juniper, this can be done using the `Nullable` type:

```rust
# extern crate juniper;
use juniper::{FieldResult, Nullable};

#[derive(juniper::GraphQLInputObject)]
struct UserPatchInput {
    pub favorite_number: Nullable<i32>,
    pub least_favorite_number: Nullable<i32>,
}

impl Into<UserPatch> for UserPatchInput {
    fn into(self) -> UserPatch {
        UserPatch {
            // The `explicit` function transforms the `Nullable` into an
            // `Option<Option<T>>` as expected by the business logic layer.
            favorite_number: self.favorite_number.explicit(),
            least_favorite_number: self.least_favorite_number.explicit(),
        }
    }
}

# pub struct UserPatch {
#     pub favorite_number: Option<Option<i32>>,
#     pub least_favorite_number: Option<Option<i32>>,
# }

# struct Session;
# impl Session {
#     fn patch_user(&self, _patch: UserPatch) -> FieldResult<()> { Ok(()) }
# }

struct Context {
    session: Session,
}

struct Mutation;

#[juniper::graphql_object(Context=Context)]
impl Mutation {
    fn patch_user(ctx: &Context, patch: UserPatchInput) -> FieldResult<bool> {
        ctx.session.patch_user(patch.into())?;
        Ok(true)
    }
}
# fn main() {}
```

This type functions much like `Option`, but has two empty variants so you can
distinguish between implicit and explicit null.
